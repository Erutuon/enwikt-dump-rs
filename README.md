# enwikt-dump-rs

This program generates data from the `pages-articles.xml` file from the [English Wiktionary](https://en.wiktionary.org/) dump.

## Subcommands

### `add-template-redirects`

Looks up the redirects to a set of templates in a file and generates a new file containing the redirects, suitable for the `dump-parsed-templates` or `dump-templates` subcommands.

### `all-headers`

Counts how many times each header appears at each header level and outputs JSON.

### `dump-parsed-templates`

Generates dumps of parsed templates containing [CBOR](https://cbor.io/)-encoded objects with the title of a page and all the instances of a given template (with the template name, parsed parameters, and the template wikitext) found on that page. This makes it faster to search template instances with a script.

### `dump-templates`

Dumps template instances in an ad-hoc format.

### `filter-headers`

Gathers the titles of all pages that contain certain headers and outputs JSON.

## Installation

Download the repository, ensure you have [cargo](https://doc.rust-lang.org/stable/cargo/) installed, `cd` to the directory, and do `cargo build --release`.