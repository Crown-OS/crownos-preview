use crownui::kit::icons;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Highlighter,
    Line,
    Arrow,
    Rectangle,
    Ellipse,
    Text,
    Crop,
}

impl Tool {
    pub const ALL: [Self; 8] = [
        Self::Pen,
        Self::Highlighter,
        Self::Line,
        Self::Arrow,
        Self::Rectangle,
        Self::Ellipse,
        Self::Text,
        Self::Crop,
    ];

    pub fn icon(self) -> &'static str {
        match self {
            Self::Pen => icons::PEN_LINE,
            Self::Highlighter => icons::HIGHLIGHTER,
            Self::Line => icons::MINUS,
            Self::Arrow => icons::ARROW_UP_RIGHT,
            Self::Rectangle => icons::SQUARE,
            Self::Ellipse => icons::CIRCLE,
            Self::Text => icons::TYPE,
            Self::Crop => icons::CROP,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pen => "Sketch",
            Self::Highlighter => "Highlight",
            Self::Line => "Line",
            Self::Arrow => "Arrow",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Oval",
            Self::Text => "Text",
            Self::Crop => "Crop",
        }
    }
}
