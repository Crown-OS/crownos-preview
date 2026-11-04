use crownui::ext::KeyboardEvent;
use crownui::prelude::{Cx, Signal};

type Handler = Box<dyn FnMut(&mut Cx, &KeyboardEvent) -> bool>;

/// The shown document's keyboard shortcuts. Keys reach only the focused
/// element and the root, so the root asks the document first.
#[derive(Default)]
pub struct Shortcuts(Option<Handler>);

pub fn provide_shortcuts(cx: &mut Cx) -> Signal<Shortcuts> {
    cx.provide(Shortcuts::default())
}

/// Makes `handler` the document's shortcuts; it returns whether it used the key.
pub fn on_shortcut(cx: &mut Cx, handler: impl FnMut(&mut Cx, &KeyboardEvent) -> bool + 'static) {
    if let Some(shortcuts) = cx.environment::<Shortcuts>() {
        shortcuts.set(cx, Shortcuts(Some(Box::new(handler))));
    }
}

/// Offers `event` to the document's shortcuts; whether one used it.
pub fn dispatch_shortcut(cx: &mut Cx, shortcuts: Signal<Shortcuts>, event: &KeyboardEvent) -> bool {
    let mut taken = None;
    shortcuts.update(cx, |shortcuts| taken = shortcuts.0.take());
    let Some(mut handler) = taken else {
        return false;
    };
    let handled = handler(cx, event);
    shortcuts.update(cx, |shortcuts| {
        shortcuts.0.get_or_insert(handler);
    });
    handled
}
