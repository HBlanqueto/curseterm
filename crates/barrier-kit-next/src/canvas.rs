//! A leaf element that hands its area to a painter that draws with Skia.

use std::any::Any;
use std::borrow::Cow;
use std::rc::Rc;

use freya_core::data::LayoutData;
use freya_core::element::{ElementExt, RenderContext};
use freya_core::prelude::*;
use freya_core::tree::DiffModifies;
use freya_engine::prelude::{Canvas as SkCanvas, FontCollection, Rect as SkRect};
use torin::prelude::Size;

pub struct CanvasContext<'a> {
    pub canvas: &'a SkCanvas,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub fonts: &'a mut FontCollection,
}

type Painter = Rc<dyn Fn(&mut CanvasContext)>;

struct Canvas {
    revision: u64,
    painter: Painter,
    layout: LayoutData,
}

pub fn canvas(revision: u64, painter: impl Fn(&mut CanvasContext) + 'static) -> Element {
    let layout = torin::node::Node {
        width: Size::fill(),
        height: Size::fill(),
        ..Default::default()
    };
    Element::Element {
        key: DiffKey::None,
        element: Rc::new(Canvas {
            revision,
            painter: Rc::new(painter),
            layout: layout.into(),
        }),
        elements: Vec::new(),
    }
}

impl ElementExt for Canvas {
    fn changed(&self, other: &Rc<dyn ElementExt>) -> bool {
        !self.diff(other).is_empty()
    }

    fn diff(&self, other: &Rc<dyn ElementExt>) -> DiffModifies {
        match (other.as_ref() as &dyn Any).downcast_ref::<Canvas>() {
            Some(other) if other.revision == self.revision => DiffModifies::empty(),
            Some(_) => DiffModifies::STYLE,
            None => DiffModifies::all(),
        }
    }

    fn layout(&'_ self) -> Cow<'_, LayoutData> {
        Cow::Borrowed(&self.layout)
    }

    fn render(&self, context: RenderContext) {
        let area = context.layout_node.visible_area();
        let scale = context.scale_factor as f32;
        let canvas = context.canvas;
        canvas.save();
        canvas.clip_rect(
            SkRect::new(area.min_x(), area.min_y(), area.max_x(), area.max_y()),
            None,
            Some(true),
        );
        canvas.translate((area.min_x(), area.min_y()));
        canvas.scale((scale, scale));
        (self.painter)(&mut CanvasContext {
            canvas,
            width: area.width() / scale,
            height: area.height() / scale,
            scale,
            fonts: context.font_collection,
        });
        canvas.restore();
    }
}
