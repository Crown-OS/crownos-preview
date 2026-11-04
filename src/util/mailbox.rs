use crownui::prelude::Cx;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

const BURST: usize = 32;

/// A channel whose messages are handled on the UI thread for as long as the
/// calling component lives.
pub fn mailbox<M: Send + 'static>(
    cx: &mut Cx,
    on_message: impl FnMut(&mut Cx, M) + 'static,
) -> UnboundedSender<M> {
    let (sender, receiver) = unbounded_channel();
    pump(cx, receiver, on_message);
    sender
}

fn pump<M: Send + 'static>(
    cx: &mut Cx,
    mut receiver: UnboundedReceiver<M>,
    mut on_message: impl FnMut(&mut Cx, M) + 'static,
) {
    let next_burst = async move {
        let mut burst = Vec::with_capacity(BURST);
        let open = receiver.recv_many(&mut burst, BURST).await > 0;
        (receiver, burst, open)
    };
    cx.spawn(next_burst, move |cx, (receiver, burst, open)| {
        burst
            .into_iter()
            .for_each(|message| on_message(cx, message));
        if open {
            pump(cx, receiver, on_message);
        }
    });
}
