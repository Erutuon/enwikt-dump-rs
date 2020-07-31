pub use parse_mediawiki_dump::Error;
pub use parse_wiki_text::{
    self, Configuration, ConfigurationSource, Node, Parameter, Positioned,
    Warning,
};
use std::io::{BufReader, Read};

mod namespaces;
pub use namespaces::Namespace;

pub type DumpParser<R> = parse_mediawiki_dump::Parser<BufReader<R>, Namespace>;

pub type Page = parse_mediawiki_dump::Page<Namespace>;

pub fn parse<R: Read>(dump_file: R) -> DumpParser<R> {
    let reader = BufReader::new(dump_file);
    parse_mediawiki_dump::parse_with_namespace(reader)
}

// Created using https://github.com/portstrom/fetch_mediawiki_configuration
pub fn wiktionary_configuration() -> Configuration {
    Configuration::new(&ConfigurationSource {
        category_namespaces: &["cat", "category"],
        extension_tags: &[
            "categorytree",
            "ce",
            "charinsert",
            "chem",
            "dynamicpagelist",
            "gallery",
            "graph",
            "hiero",
            "imagemap",
            "indicator",
            "inputbox",
            "mapframe",
            "maplink",
            "math",
            "nowiki",
            "poem",
            "pre",
            "ref",
            "references",
            "score",
            "section",
            "source",
            "syntaxhighlight",
            "talkpage",
            "templatedata",
            "templatestyles",
            "thread",
            "timeline",
        ],
        file_namespaces: &["file", "image"],
        link_trail: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        magic_words: &[
            "DISAMBIG",
            "EXPECTUNUSEDCATEGORY",
            "FORCETOC",
            "HIDDENCAT",
            "INDEX",
            "NEWSECTIONLINK",
            "NOCC",
            "NOCOLLABORATIONHUBTOC",
            "NOCONTENTCONVERT",
            "NOEDITSECTION",
            "NOGALLERY",
            "NOGLOBAL",
            "NOINDEX",
            "NONEWSECTIONLINK",
            "NOTC",
            "NOTITLECONVERT",
            "NOTOC",
            "STATICREDIRECT",
            "TOC",
        ],
        protocols: &[
            "//",
            "bitcoin:",
            "ftp://",
            "ftps://",
            "geo:",
            "git://",
            "gopher://",
            "http://",
            "https://",
            "irc://",
            "ircs://",
            "magnet:",
            "mailto:",
            "mms://",
            "news:",
            "nntp://",
            "redis://",
            "sftp://",
            "sip:",
            "sips:",
            "sms:",
            "ssh://",
            "svn://",
            "tel:",
            "telnet://",
            "urn:",
            "worldwind://",
            "xmpp:",
        ],
        redirect_magic_words: &["REDIRECT"],
    })
}
