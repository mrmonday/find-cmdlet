# Find-Cmdlet

A search engine for PowerShell cmdlets.

Source code for: https://find-cmdlet.com/

## Source Overview

There are three main components:

 1. Scraper
 2. Indexer
 3. The site

### Scraper

The scraper pulls cmdlet data from three locations:

 * Built in Windows Powershell snapins
 * RSAT Windows features
 * PowerShell Gallery

Each of these is handled by a Docker container so they can be handled in
relative isolation.

With each module loaded, the scraper is then updates the help text, and runs
`Get-Module`, `Get-Command`, and `Get-Help`, dumping the results with
`ConvertTo-Json`. The resulting json can then be processed by the indexer.

### Indexer

The indexer takes the json output from the scraper, and feeds it into
[Tantivy](https://github.com/tantivy-search/tantivy). Currently anything
clever is left to Tantivy.

### The site

This is what you see at https://find-cmdlet.com/. It's a basic front end on top
of the index produced by Tantivy.
