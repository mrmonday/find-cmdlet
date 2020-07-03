use tantivy::Index;
use tantivy::tokenizer::{BoxTokenStream, Token, TokenFilter, TokenStream};
use voca_rs::Voca;

struct PascalSplitterStream<'a> {
    tail: BoxTokenStream<'a>,
    tokens: Vec<String>,
    token: Token,
    token_count: usize,
    additional_tokens: usize,
}

impl<'a> PascalSplitterStream<'a> {
    fn new(stream: BoxTokenStream<'a>) -> Self {
        PascalSplitterStream {
            tail: stream,
            tokens: Vec::new(),
            token: Token::default(),
            token_count: 0,
            additional_tokens: 0,
        }
    }
}

impl<'a> TokenStream for PascalSplitterStream<'a> {
    fn advance(&mut self) -> bool {
        if self.tokens.is_empty() {
            if !self.tail.advance() {
                return false;
            }

            self.tokens = self
                .tail
                .token()
                .text
                ._words()
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            self.additional_tokens = self.tokens.len() - 1;

            if self.tokens.is_empty() {
                return false;
            }
        }
        let token = self.tokens.remove(0);
        let mut tail_token = self.tail.token_mut();
        self.token.offset_from = tail_token.offset_from;
        self.token.offset_to = tail_token.offset_from + token.len();
        self.token.position = tail_token.position + self.token_count;
        self.token.position_length = tail_token.position_length;
        self.token.text = token;
        if self.additional_tokens > 0 {
            self.token_count += 1;
            self.additional_tokens -= 1;
        }

        if !self.tokens.is_empty() {
            let next_token_index = tail_token.text[self.token.text.len()..]
                .find(&self.tokens[0])
                .unwrap()
                + self.token.text.len();
            tail_token.text = tail_token.text.split_off(next_token_index);
            tail_token.offset_from += next_token_index;
        }

        true
    }
    fn token(&self) -> &Token {
        &self.token
    }
    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}

#[derive(Clone)]
pub struct PascalSplitter;

impl TokenFilter for PascalSplitter {
    fn transform<'a>(&self, token_stream: BoxTokenStream<'a>) -> BoxTokenStream<'a> {
        BoxTokenStream::from(PascalSplitterStream::new(token_stream))
    }
}

pub fn register(index: &Index) {
    let pascal_tokenizer =
        tantivy::tokenizer::TextAnalyzer::from(tantivy::tokenizer::SimpleTokenizer)
            .filter(PascalSplitter)
            .filter(tantivy::tokenizer::LowerCaser);

    index
        .tokenizers()
        .register("pascal", pascal_tokenizer);
}

#[test]
fn cmdlet_split() {
    use tantivy::tokenizer::{SimpleTokenizer, TextAnalyzer};

    let tokenizer = TextAnalyzer::from(SimpleTokenizer).filter(PascalSplitter);
    let test_cases: &[(&str, &[_])] = &[
        ("New-VM", &[(0, 3, "New"), (4, 6, "VM")]),
        (
            "Set-HTMLContent",
            &[(0, 3, "Set"), (4, 8, "HTML"), (8, 15, "Content")],
        ),
        (
            "Set-HtmlContent",
            &[(0, 3, "Set"), (4, 8, "Html"), (8, 15, "Content")],
        ),
        ("Set-Html", &[(0, 3, "Set"), (4, 8, "Html")]),
        ("Set-HTML", &[(0, 3, "Set"), (4, 8, "HTML")]),
    ];

    for (cmdlet, res) in test_cases {
        let mut stream = tokenizer.token_stream(cmdlet);
        let mut i = 0;
        while stream.advance() {
            let (expected_offset_from, expected_offset_to, expected_str) = res[i];
            let token = stream.token();
            assert_eq!(token.offset_from, expected_offset_from);
            assert_eq!(token.offset_to, expected_offset_to);
            assert_eq!(token.position, i);
            assert_eq!(token.text, expected_str);
            i += 1;
        }
    }
}
