use std::convert::{From, TryFrom};
use std::str::{self, FromStr};

#[derive(Copy, Clone, Eq, Debug, Hash, PartialEq)]
pub enum Namespace {
    /*
    Media,
    Special,
    */
    Main,
    Talk,
    User,
    UserTalk,
    Wiktionary,
    WiktionaryTalk,
    File,
    FileTalk,
    MediaWiki,
    MediaWikiTalk,
    Template,
    TemplateTalk,
    Help,
    HelpTalk,
    Category,
    CategoryTalk,
    Thread,
    ThreadTalk,
    Summary,
    SummaryTalk,
    Appendix,
    AppendixTalk,
    Concordance,
    ConcordanceTalk,
    Index,
    IndexTalk,
    Rhymes,
    RhymesTalk,
    Transwiki,
    TranswikiTalk,
    Thesaurus,
    ThesaurusTalk,
    Citations,
    CitationsTalk,
    SignGloss,
    SignGlossTalk,
    Reconstruction,
    ReconstructionTalk,
    Module,
    ModuleTalk,
    Gadget,
    GadgetTalk,
    GadgetDefinition,
    GadgetDefinitionTalk,
}

impl Namespace {
    pub const MAX_LEN: usize = 22;
    
    pub fn as_str (&self) -> &'static str {
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
}

impl TryFrom<u32> for Namespace {
    type Error = &'static str;
    
    fn try_from (id: u32) -> Result<Self, Self::Error> {
        let namespace = match id {
              /*
              -2 => Namespace::Media,
              -1 => Namespace::Special,
              */
               0 => Namespace::Main,
               1 => Namespace::Talk,
               2 => Namespace::User,
               3 => Namespace::UserTalk,
               4 => Namespace::Wiktionary,
               5 => Namespace::WiktionaryTalk,
               6 => Namespace::File,
               7 => Namespace::FileTalk,
               8 => Namespace::MediaWiki,
               9 => Namespace::MediaWikiTalk,
              10 => Namespace::Template,
              11 => Namespace::TemplateTalk,
              12 => Namespace::Help,
              13 => Namespace::HelpTalk,
              14 => Namespace::Category,
              15 => Namespace::CategoryTalk,
              90 => Namespace::Thread,
              91 => Namespace::ThreadTalk,
              92 => Namespace::Summary,
              93 => Namespace::SummaryTalk,
             100 => Namespace::Appendix,
             101 => Namespace::AppendixTalk,
             102 => Namespace::Concordance,
             103 => Namespace::ConcordanceTalk,
             104 => Namespace::Index,
             105 => Namespace::IndexTalk,
             106 => Namespace::Rhymes,
             107 => Namespace::RhymesTalk,
             108 => Namespace::Transwiki,
             109 => Namespace::TranswikiTalk,
             110 => Namespace::Thesaurus,
             111 => Namespace::ThesaurusTalk,
             114 => Namespace::Citations,
             115 => Namespace::CitationsTalk,
             116 => Namespace::SignGloss,
             117 => Namespace::SignGlossTalk,
             118 => Namespace::Reconstruction,
             119 => Namespace::ReconstructionTalk,
             828 => Namespace::Module,
             829 => Namespace::ModuleTalk,
            2300 => Namespace::Gadget,
            2301 => Namespace::GadgetTalk,
            2302 => Namespace::GadgetDefinition,
            2303 => Namespace::GadgetDefinitionTalk,
               _ => return Err("invalid namespace id"),
        };
        Ok(namespace)
    }
}

impl From<Namespace> for u32 {
    fn from (namespace: Namespace) -> Self {
        match namespace {
            /*
            Namespace::Media                =>   -2,
            Namespace::Special              =>   -1,
            */
            Namespace::Main                 =>    0,
            Namespace::Talk                 =>    1,
            Namespace::User                 =>    2,
            Namespace::UserTalk             =>    3,
            Namespace::Wiktionary           =>    4,
            Namespace::WiktionaryTalk       =>    5,
            Namespace::File                 =>    6,
            Namespace::FileTalk             =>    7,
            Namespace::MediaWiki            =>    8,
            Namespace::MediaWikiTalk        =>    9,
            Namespace::Template             =>   10,
            Namespace::TemplateTalk         =>   11,
            Namespace::Help                 =>   12,
            Namespace::HelpTalk             =>   13,
            Namespace::Category             =>   14,
            Namespace::CategoryTalk         =>   15,
            Namespace::Thread               =>   90,
            Namespace::ThreadTalk           =>   91,
            Namespace::Summary              =>   92,
            Namespace::SummaryTalk          =>   93,
            Namespace::Appendix             =>  100,
            Namespace::AppendixTalk         =>  101,
            Namespace::Concordance          =>  102,
            Namespace::ConcordanceTalk      =>  103,
            Namespace::Index                =>  104,
            Namespace::IndexTalk            =>  105,
            Namespace::Rhymes               =>  106,
            Namespace::RhymesTalk           =>  107,
            Namespace::Transwiki            =>  108,
            Namespace::TranswikiTalk        =>  109,
            Namespace::Thesaurus            =>  110,
            Namespace::ThesaurusTalk        =>  111,
            Namespace::Citations            =>  114,
            Namespace::CitationsTalk        =>  115,
            Namespace::SignGloss            =>  116,
            Namespace::SignGlossTalk        =>  117,
            Namespace::Reconstruction       =>  118,
            Namespace::ReconstructionTalk   =>  119,
            Namespace::Module               =>  828,
            Namespace::ModuleTalk           =>  829,
            Namespace::Gadget               => 2300,
            Namespace::GadgetTalk           => 2301,
            Namespace::GadgetDefinition     => 2302,
            Namespace::GadgetDefinitionTalk => 2303,
        }
    }
}

fn normalize_namespace_name (name: &mut [u8]) {
    name.make_ascii_lowercase();
    for c in name {
        if *c == b'_' {
            *c = b' ';
        }
    }
}

impl FromStr for Namespace {
    type Err = &'static str;
    
    // Todo: make case-insensitive and treat spaces and underscores alike.
    fn from_str (namespace_name: &str) -> Result<Self, Self::Err> {
        if namespace_name.len() > Self::MAX_LEN || !namespace_name.is_ascii() {
            return Err("invalid namespace name");
        }
        let mut namespace_buffer: [u8; Self::MAX_LEN] = [0; Self::MAX_LEN];
        let mut namespace_name_copy = &mut namespace_buffer[0..namespace_name.len()];
        namespace_name_copy.copy_from_slice(namespace_name.as_bytes());
        normalize_namespace_name(&mut namespace_name_copy);
        let namespace_name = unsafe {
            str::from_utf8_unchecked_mut(namespace_name_copy)
        };
        let namespace = match namespace_name.as_ref() {
            /*
            "media"                  => Namespace::Media,
            "special"                => Namespace::Special,
            */
            "main"                   => Namespace::Main,
            "talk"                   => Namespace::Talk,
            "user"                   => Namespace::User,
            "user talk"              => Namespace::UserTalk,
            "wiktionary"             => Namespace::Wiktionary,
            "wiktionary talk"        => Namespace::WiktionaryTalk,
            "file"                   => Namespace::File,
            "file talk"              => Namespace::FileTalk,
            "media wiki"             => Namespace::MediaWiki,
            "media wiki talk"        => Namespace::MediaWikiTalk,
            "template"               => Namespace::Template,
            "template talk"          => Namespace::TemplateTalk,
            "help"                   => Namespace::Help,
            "help talk"              => Namespace::HelpTalk,
            "category"               => Namespace::Category,
            "category talk"          => Namespace::CategoryTalk,
            "thread"                 => Namespace::Thread,
            "thread talk"            => Namespace::ThreadTalk,
            "summary"                => Namespace::Summary,
            "summary talk"           => Namespace::SummaryTalk,
            "appendix"               => Namespace::Appendix,
            "appendix talk"          => Namespace::AppendixTalk,
            "concordance"            => Namespace::Concordance,
            "concordance talk"       => Namespace::ConcordanceTalk,
            "index"                  => Namespace::Index,
            "index talk"             => Namespace::IndexTalk,
            "rhymes"                 => Namespace::Rhymes,
            "rhymes talk"            => Namespace::RhymesTalk,
            "transwiki"              => Namespace::Transwiki,
            "transwiki talk"         => Namespace::TranswikiTalk,
            "thesaurus"              => Namespace::Thesaurus,
            "thesaurus talk"         => Namespace::ThesaurusTalk,
            "citations"              => Namespace::Citations,
            "citations talk"         => Namespace::CitationsTalk,
            "sign gloss"             => Namespace::SignGloss,
            "sign gloss talk"        => Namespace::SignGlossTalk,
            "reconstruction"         => Namespace::Reconstruction,
            "reconstruction talk"    => Namespace::ReconstructionTalk,
            "module"                 => Namespace::Module,
            "module talk"            => Namespace::ModuleTalk,
            "gadget"                 => Namespace::Gadget,
            "gadget talk"            => Namespace::GadgetTalk,
            "gadget definition"      => Namespace::GadgetDefinition,
            "gadget definition talk" => Namespace::GadgetDefinitionTalk,
            _                        => return Err("invalid namespace name"),
        };
        Ok(namespace)
    }
}

#[cfg(test)]
mod tests {
    use super::Namespace;
    use std::convert::TryFrom;
    use std::str::FromStr;
    
    #[test]
    fn namespace_from_str() {
        assert_eq!(Namespace::from_str("wiktionary talk"), Ok(Namespace::WiktionaryTalk));
        assert_eq!(Namespace::from_str("Wiktionary talk"), Ok(Namespace::WiktionaryTalk));
        assert_eq!(Namespace::from_str("Wiktionary_talk"), Ok(Namespace::WiktionaryTalk));
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
        assert_eq!(Namespace::try_from(1000), Err("invalid namespace id"));
        
        assert_eq!(u32::from(Namespace::Module), 828);
        assert_eq!(u32::from(Namespace::ModuleTalk), 829);
    }
}