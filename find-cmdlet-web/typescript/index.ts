function htmlEncode(text: string): string {
    const el = document.createElement('div');
    el.innerText = text;
    return el.innerHTML;
}

function displayJson(json: [any]) {
    const results = document.querySelector<HTMLElement>('#results')!;
    let resultHTML = '';
    for (let cmdlet of json) {
        const tags = cmdlet.tags.map((t: any) => `<li>${htmlEncode(t)}</li>`).join('');
        const template = `
<div class="result">
    <div class="name"><a href="${cmdlet.url}">${htmlEncode(cmdlet.name)}</a></div>
    <div class="module">
        <span class="mod_name">${htmlEncode(cmdlet.module_name)}</span>
        (<span class="mod_version">${htmlEncode(cmdlet.module_version)}</span>)
    </div>
    <div class="tags"><ul>${tags}</ul></div>
    <div class="synopsis">${htmlEncode(cmdlet.synopsis)}</div>
</div>`;
        resultHTML += template;
    }

    if (resultHTML.length === 0) {
        const animal = String.fromCharCode(0xd83d, Math.floor(Math.random() * (0xdc3f - 0xdc00) + 0xdc00));
        const template = `<div class="result">
            No cmdlets found ${animal}
        </div>`;
        resultHTML += template;
    }

    results.innerHTML = resultHTML;
    const body = document.querySelector<HTMLBodyElement>('body');
    body?.classList.add('search');
}

function main() {
    const searchForm = document.forms.namedItem('search');

    window.onpopstate = (event: PopStateEvent) => {
        if (event.state) {
            const query = document.querySelector<HTMLInputElement>('input[name=q]');
            if (query) {
                query.value = event.state.query;
            }

            displayJson(event.state.json);
        } else {
            const body = document.querySelector<HTMLBodyElement>('body');
            body?.classList.remove('search')
            const results = document.querySelector<HTMLElement>('#results')!;
            results.innerHTML = '';
        }
    };

    searchForm?.addEventListener('submit', async function(event) {
        event.preventDefault();
        const query = this.querySelector<HTMLInputElement>('input[name=q]')?.value;
        if (!query) {
            return;
        }
        const url = '/search?t=json&q=' + encodeURIComponent(query);
        try {
            let response = await fetch(url);
            if (!response.ok) {
                throw '';
            }

            let json: [any] = await response.json();

            displayJson(json);

            window.history.pushState({query: query, json: json}, '', url);
        } catch {
            this.submit();
            return;
        }
    });

}

main();