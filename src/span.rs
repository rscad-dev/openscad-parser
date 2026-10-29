#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Span<T = usize, C: SpanContext = T> {
    pub start: T,
    pub end: T,
    pub context: C,
}

pub trait SpanContext: PartialEq + ToOwned + Clone + core::fmt::Debug {}
impl<T: PartialEq + ToOwned + Clone + core::fmt::Debug> SpanContext for T {}

impl<T, C: SpanContext> Span<T, C> {
    pub const fn context(&self) -> &C {
        &self.context
    }

    pub const fn start(&self) -> &T {
        &self.start
    }

    pub const fn end(&self) -> &T {
        &self.end
    }
}

impl<T: Clone, C: SpanContext> chumsky::span::Span for Span<T, C> {
    type Context = C;
    type Offset = T;

    fn new(context: Self::Context, range: std::ops::Range<Self::Offset>) -> Self {
        Self {
            start: range.start,
            end: range.end,
            context,
        }
    }
    fn context(&self) -> Self::Context {
        self.context.clone()
    }
    fn start(&self) -> Self::Offset {
        self.start.clone()
    }
    fn end(&self) -> Self::Offset {
        self.end.clone()
    }
}

impl<C: SpanContext> ariadne::Span for Span<usize, C> {
    type SourceId = C;

    fn source(&self) -> &Self::SourceId {
        self.context()
    }

    fn start(&self) -> usize {
        self.start
    }
    fn end(&self) -> usize {
        self.end
    }
}
