use std::time::Duration;

use crownui::prelude::Cx;

/// Calls `then` on the UI thread after `delay`, unless the calling component
/// unmounted first.
pub fn after(cx: &mut Cx, delay: Duration, then: impl FnOnce(&mut Cx) + 'static) {
    cx.spawn(
        async move { tokio::time::sleep(delay).await },
        move |cx, ()| then(cx),
    );
}
