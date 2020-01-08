pub use num_enum::TryFromPrimitiveError;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::str::FromStr;

#[derive(Copy, Clone, Eq, Debug, Hash, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u32)]
#[rustfmt::skip]
pub enum Namespace {
    /*
    Media                =   -2,
    Special              =   -1,
    */
    Main                 =    0,
    Talk                 =    1,
    User                 =    2,
    UserTalk             =    3,
    Wiktionary           =    4,
    WiktionaryTalk       =    5,
    File                 =    6,
    FileTalk             =    7,
    MediaWiki            =    8,
    MediaWikiTalk        =    9,
    Template             =   10,
    TemplateTalk         =   11,
    Help                 =   12,
    HelpTalk             =   13,
    Category             =   14,
    CategoryTalk         =   15,
    Thread               =   90,
    ThreadTalk           =   91,
    Summary              =   92,
    SummaryTalk          =   93,
    Appendix             =  100,
    AppendixTalk         =  101,
    Concordance          =  102,
    ConcordanceTalk      =  103,
    Index                =  104,
    IndexTalk            =  105,
    Rhymes               =  106,
    RhymesTalk           =  107,
    Transwiki            =  108,
    TranswikiTalk        =  109,
    Thesaurus            =  110,
    ThesaurusTalk        =  111,
    Citations            =  114,
    CitationsTalk        =  115,
    SignGloss            =  116,
    SignGlossTalk        =  117,
    Reconstruction       =  118,
    ReconstructionTalk   =  119,
    Module               =  828,
    ModuleTalk           =  829,
    Gadget               = 2300,
    GadgetTalk           = 2301,
    GadgetDefinition     = 2302,
    GadgetDefinitionTalk = 2303,
}

impl Namespace {
    pub const MAX_LEN: usize = 22;

    #[rustfmt::skip]
    pub fn as_str(&self) -> &'static str {
        match &self {
            /*
            Namespace::Media                => "Media",
            Namespace::Special              => "Special",
            */
            Namespace::Main                 => "",
            Namespace::Talk                 => "Talk",
            Namespace::User                 => "User",
            Namespace::UserTalk             => "User talk",
            Namespace::Wiktionary           => "Wiktionary",
            Namespace::WiktionaryTalk       => "Wiktionary talk",
            Namespace::File                 => "File",
            Namespace::FileTalk             => "File talk",
            Namespace::MediaWiki            => "MediaWiki",
            Namespace::MediaWikiTalk        => "MediaWiki talk",
            Namespace::Template             => "Template",
            Namespace::TemplateTalk         => "Template talk",
            Namespace::Help                 => "Help",
            Namespace::HelpTalk             => "Help talk",
            Namespace::Category             => "Category",
            Namespace::CategoryTalk         => "Category talk",
            Namespace::Thread               => "Thread",
            Namespace::ThreadTalk           => "Thread talk",
            Namespace::Summary              => "Summary",
            Namespace::SummaryTalk          => "Summary talk",
            Namespace::Appendix             => "Appendix",
            Namespace::AppendixTalk         => "Appendix talk",
            Namespace::Concordance          => "Concordance",
            Namespace::ConcordanceTalk      => "Concordance talk",
            Namespace::Index                => "Index",
            Namespace::IndexTalk            => "Index talk",
            Namespace::Rhymes               => "Rhymes",
            Namespace::RhymesTalk           => "Rhymes talk",
            Namespace::Transwiki            => "Transwiki",
            Namespace::TranswikiTalk        => "Transwiki talk",
            Namespace::Thesaurus            => "Thesaurus",
            Namespace::ThesaurusTalk        => "Thesaurus talk",
            Namespace::Citations            => "Citations",
            Namespace::CitationsTalk        => "Citations talk",
            Namespace::SignGloss            => "Sign gloss",
            Namespace::SignGlossTalk        => "Sign gloss talk",
            Namespace::Reconstruction       => "Reconstruction",
            Namespace::ReconstructionTalk   => "Reconstruction talk",
            Namespace::Module               => "Module",
            Namespace::ModuleTalk           => "Module talk",
            Namespace::Gadget               => "Gadget",
            Namespace::GadgetTalk           => "Gadget talk",
            Namespace::GadgetDefinition     => "Gadget definition",
            Namespace::GadgetDefinitionTalk => "Gadget definition talk",
        }
    }

    pub fn normalize_name<'a>(name: &str, buffer: &'a mut [u8]) -> &'a str {
        let normalized_name = &mut buffer[..name.len()];
        normalized_name.copy_from_slice(name.as_bytes());
        normalized_name[0] = normalized_name[0].to_ascii_uppercase();
        normalized_name[1..].make_ascii_lowercase();
        for c in normalized_name.iter_mut() {
            if *c == b'_' {
                *c = b' ';
            }
        }
        unsafe { std::str::from_utf8_unchecked(&*normalized_name) }
    }
}

impl FromStr for Namespace {
    type Err = &'static str;

    fn from_str(namespace_name: &str) -> Result<Self, Self::Err> {
        if namespace_name.len() > Self::MAX_LEN || !namespace_name.is_ascii() {
            return Err("invalid namespace name");
        }
        let mut namespace_buffer = [0u8; Self::MAX_LEN];
        let namespace_name =
            Self::normalize_name(namespace_name, &mut namespace_buffer);
        #[rustfmt::skip]
        let namespace = match namespace_name.as_ref() {
            /*
            "Media"                  => Namespace::Media,
            "Special"                => Namespace::Special,
            */
            "Main"                   => Namespace::Main,
            "Talk"                   => Namespace::Talk,
            "User"                   => Namespace::User,
            "User talk"              => Namespace::UserTalk,
            "Wiktionary"             => Namespace::Wiktionary,
            "Wiktionary talk"        => Namespace::WiktionaryTalk,
            "File"                   => Namespace::File,
            "File talk"              => Namespace::FileTalk,
            "Media wiki"             => Namespace::MediaWiki,
            "Media wiki talk"        => Namespace::MediaWikiTalk,
            "Template"               => Namespace::Template,
            "Template talk"          => Namespace::TemplateTalk,
            "Help"                   => Namespace::Help,
            "Help talk"              => Namespace::HelpTalk,
            "Category"               => Namespace::Category,
            "Category talk"          => Namespace::CategoryTalk,
            "Thread"                 => Namespace::Thread,
            "Thread talk"            => Namespace::ThreadTalk,
            "Summary"                => Namespace::Summary,
            "Summary talk"           => Namespace::SummaryTalk,
            "Appendix"               => Namespace::Appendix,
            "Appendix talk"          => Namespace::AppendixTalk,
            "Concordance"            => Namespace::Concordance,
            "Concordance talk"       => Namespace::ConcordanceTalk,
            "Index"                  => Namespace::Index,
            "Index talk"             => Namespace::IndexTalk,
            "Rhymes"                 => Namespace::Rhymes,
            "Rhymes talk"            => Namespace::RhymesTalk,
            "Transwiki"              => Namespace::Transwiki,
            "Transwiki talk"         => Namespace::TranswikiTalk,
            "Thesaurus"              => Namespace::Thesaurus,
            "Thesaurus talk"         => Namespace::ThesaurusTalk,
            "Citations"              => Namespace::Citations,
            "Citations talk"         => Namespace::CitationsTalk,
            "Sign gloss"             => Namespace::SignGloss,
            "Sign gloss talk"        => Namespace::SignGlossTalk,
            "Reconstruction"         => Namespace::Reconstruction,
            "Reconstruction talk"    => Namespace::ReconstructionTalk,
            "Module"                 => Namespace::Module,
            "Module talk"            => Namespace::ModuleTalk,
            "Gadget"                 => Namespace::Gadget,
            "Gadget talk"            => Namespace::GadgetTalk,
            "Gadget definition"      => Namespace::GadgetDefinition,
            "Gadget definition talk" => Namespace::GadgetDefinitionTalk,
            _                        => return Err("invalid namespace name"),
        };
        Ok(namespace)
    }
}

#[cfg(test)]
mod tests {
    use super::{Namespace, TryFromPrimitiveError};
    use std::convert::TryFrom;
    use std::str::FromStr;

    #[test]
    fn namespace_from_str() {
        assert_eq!(
            Namespace::from_str("wiktionary talk"),
            Ok(Namespace::WiktionaryTalk)
        );
        assert_eq!(
            Namespace::from_str("Wiktionary talk"),
            Ok(Namespace::WiktionaryTalk)
        );
        assert_eq!(
            Namespace::from_str("Wiktionary_talk"),
            Ok(Namespace::WiktionaryTalk)
        );
    }

    #[test]
    fn namespace_as_str() {
        assert_eq!(Namespace::Talk.as_str(), "Talk");
        assert_eq!(Namespace::WiktionaryTalk.as_str(), "Wiktionary talk");
    }

    #[test]
    fn namespace_numbers() {
        assert_eq!(Namespace::try_from(828), Ok(Namespace::Module));
        assert_eq!(Namespace::try_from(829), Ok(Namespace::ModuleTalk));
        assert_eq!(
            Namespace::try_from(1000),
            Err(TryFromPrimitiveError { number: 1000u32 })
        );

        assert_eq!(u32::from(Namespace::Module), 828);
        assert_eq!(u32::from(Namespace::ModuleTalk), 829);
    }

    #[test]
    fn normalize_namespace_name() {}
}
