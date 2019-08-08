use std::io::{BufReader, Read};
use parse_mediawiki_dump;
use parse_wiki_text::{Configuration, ConfigurationSource};

pub fn parse<R: Read>(dump: R) -> parse_mediawiki_dump::Parser<BufReader<R>> {
    let reader = BufReader::new(dump);
    parse_mediawiki_dump::parse(reader)
}

// Created using https://github.com/portstrom/fetch_mediawiki_configuration
pub fn wiktionary_configuration() -> Configuration {
    Configuration::new(&ConfigurationSource {
        category_namespaces: &[
            "cat",
            "category",
        ],
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
        file_namespaces: &[
            "file",
            "image",
        ],
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
        redirect_magic_words: &[
            "REDIRECT",
        ]
    })
}