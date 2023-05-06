
#[derive(Debug)]
pub enum DeferralKind {
    StyleLink,
    StyleInline,
    StyleAttr,
    Source,
    ImageLink,
    UnquotedUrl,
    QuotedUrl,
}


pub struct Deferral {
    pub kind: DeferralKind,
    pub i: usize,
    pub data: String
}


impl DeferralKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeferralKind::ImageLink => "ImageLink",
            DeferralKind::QuotedUrl => "QuotedUrl",
            DeferralKind::Source => "Source",
            DeferralKind::StyleAttr => "StyleAttr",
            DeferralKind::StyleInline => "StyleInline",
            DeferralKind::StyleLink => "StyleLink",
            DeferralKind::UnquotedUrl => "UnquotedUrl",
        }
    }
}
