use iced::Element;

/// A custom renderer for code blocks with specific languages.
///
/// Implement this trait to provide custom rendering for languages like
/// mermaid, vega-lite, or any other specialized code block format.
pub trait CustomBlockRenderer<Message, Theme, Renderer> {
    /// Returns the language identifiers this renderer handles.
    fn languages(&self) -> &[&str];

    /// Renders a code block for the given language.
    ///
    /// `is_streaming` is `true` while the code fence is still being written.
    /// Most renderers should show a placeholder until streaming is complete.
    fn render<'a>(
        &self,
        language: &str,
        code: &str,
        is_streaming: bool,
    ) -> Element<'a, Message, Theme, Renderer>;
}

/// A registry of custom code block renderers.
///
/// Renderers are checked in order; the first one that handles a given
/// language wins.
pub struct CustomRendererRegistry<Message, Theme, Renderer> {
    renderers: Vec<Box<dyn CustomBlockRenderer<Message, Theme, Renderer>>>,
}

impl<Message, Theme, Renderer> Default for CustomRendererRegistry<Message, Theme, Renderer> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message, Theme, Renderer> CustomRendererRegistry<Message, Theme, Renderer> {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            renderers: Vec::new(),
        }
    }

    /// Adds a custom renderer to the registry.
    pub fn add(
        mut self,
        renderer: impl CustomBlockRenderer<Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.renderers.push(Box::new(renderer));
        self
    }

    /// Attempts to render a code block with a registered custom renderer.
    ///
    /// Returns `None` if no renderer handles the given language.
    pub fn try_render<'a>(
        &self,
        language: &str,
        code: &str,
        is_streaming: bool,
    ) -> Option<Element<'a, Message, Theme, Renderer>> {
        self.renderers.iter().find_map(|r| {
            if r.languages().iter().any(|l| *l == language) {
                Some(r.render(language, code, is_streaming))
            } else {
                None
            }
        })
    }
}
