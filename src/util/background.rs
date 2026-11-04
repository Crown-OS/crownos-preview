use crownui::prelude::Cx;

/// Runs blocking `work` on its own thread and hands its result to `on_done`
/// on the UI thread, unless the calling component unmounted first.
pub fn in_background<T: Send + 'static>(
    cx: &mut Cx,
    work: impl FnOnce() -> T + Send + 'static,
    on_done: impl FnOnce(&mut Cx, T) + 'static,
) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || sender.send(work()));
    cx.spawn(receiver, move |cx, result| {
        if let Ok(value) = result {
            on_done(cx, value);
        }
    });
}
