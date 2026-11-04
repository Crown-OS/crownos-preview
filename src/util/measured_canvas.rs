use crownui::ext::{DrawList, Kind, NodeId};
use crownui::prelude::{Cx, Element, Runtime, Signal, Size};

pub struct MeasuredCanvasKind<P> {
    size: Signal<Size>,
    painter: P,
}

impl<P: FnMut(&mut Runtime, Size, &mut DrawList) + 'static> Kind for MeasuredCanvasKind<P> {
    const NAME: &'static str = "measured_canvas";

    fn mount(self, cx: &mut Cx, node: NodeId) {
        cx.set_canvas(node, self.painter);
        let observed = cx.observe_size(node);
        let size = self.size;
        cx.effect(move |runtime| {
            let current = observed.get(runtime);
            size.set(runtime, current);
        });
    }
}

/// A canvas that keeps `size` in step with its laid-out size, so handlers and
/// bindings outside the painter can map between view and content coordinates.
pub fn measured_canvas<P: FnMut(&mut Runtime, Size, &mut DrawList) + 'static>(
    size: Signal<Size>,
    painter: P,
) -> Element<MeasuredCanvasKind<P>> {
    Element::new(MeasuredCanvasKind { size, painter })
}
