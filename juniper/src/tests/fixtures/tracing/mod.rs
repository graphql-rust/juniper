//! Fixtures used to test integration with [`tracing`] crate.
//!
//! [`tracing`]: crate::tracing

pub mod schema;

use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use tracing_core::{span, Subscriber};

use crate::tracing::{
    self,
    field::{Field, Visit},
    span::{Attributes, Record},
    Event, Level, Metadata,
};

/// Information about `tracing` span recorded within tests.
#[derive(Clone, Debug)]
struct TestSpan {
    id: span::Id,
    fields: HashMap<String, String>,
    metadata: &'static Metadata<'static>,
}

/// Information about `tracing` event recorded within tests.
#[derive(Clone, Debug)]
pub struct TestEvent {
    fields: HashMap<String, String>,
    metadata: &'static Metadata<'static>,
}

/// Method calls on [`TestSubscriber`].
#[derive(Clone, Debug)]
enum SubscriberEvent {
    /// `new_span` method.
    NewSpan(TestSpan),

    /// `enter` method.
    Enter(span::Id),

    /// `exit` method.
    Exit(span::Id),

    /// `clone_span` method.
    CloneSpan(span::Id),

    /// `try_close` method.
    TryClose(span::Id),

    /// `event` method.
    Event(TestEvent),
}

impl TestEvent {
    /// Constructs new [`TestEvent`] from `tracing` [`Event`].
    pub fn new(ev: &Event<'_>) -> Self {
        let mut visitor = Visitor::new();

        ev.record(&mut visitor);
        Self {
            fields: visitor.0,
            metadata: ev.metadata(),
        }
    }
}

/// Simple visitor useful for converting `tracing` [`Event`]s into [`TestEvent`]s.
struct Visitor(HashMap<String, String>);

impl Visitor {
    fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0
            .insert(field.name().to_owned(), format!("{:?}", value));
    }
}

/// Subscriber that logs every method call.
#[derive(Clone)]
pub struct TestSubscriber {
    /// Counter used to create unique [`span::Id`]s.
    counter: Rc<RefCell<u64>>,

    /// Log of method calls to this subscriber.
    events: Rc<RefCell<Vec<SubscriberEvent>>>,
}

unsafe impl Sync for TestSubscriber {}

unsafe impl Send for TestSubscriber {}

impl TestSubscriber {
    /// Returns new [`TestSubscriber`].
    pub fn new() -> Self {
        TestSubscriber {
            counter: Rc::new(RefCell::new(1)),
            events: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Creates [`SubscriberAssert`] used to validated constructed spans.
    pub fn assert(self) -> SubscriberAssert {
        SubscriberAssert {
            name_to_span: HashMap::new(),
            events: self.events.borrow().clone(),
        }
    }
}

impl Subscriber for TestSubscriber {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attrs: &Attributes<'_>) -> span::Id {
        let id = *self.counter.borrow();
        *self.counter.borrow_mut() = id + 1;

        let mut visitor = Visitor::new();
        attrs.record(&mut visitor);

        let id = span::Id::from_u64(id);
        let test_span = TestSpan {
            id: id.clone(),
            metadata: attrs.metadata(),
            fields: visitor.0,
        };
        self.events
            .borrow_mut()
            .push(SubscriberEvent::NewSpan(test_span));
        id
    }

    fn record(&self, _: &span::Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, event: &Event<'_>) {
        self.events
            .borrow_mut()
            .push(SubscriberEvent::Event(TestEvent::new(event)))
    }

    fn enter(&self, id: &span::Id) {
        self.events
            .borrow_mut()
            .push(SubscriberEvent::Enter(id.clone()))
    }

    fn exit(&self, id: &span::Id) {
        self.events
            .borrow_mut()
            .push(SubscriberEvent::Exit(id.clone()))
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.events
            .borrow_mut()
            .push(SubscriberEvent::CloneSpan(id.clone()));
        id.clone()
    }

    fn try_close(&self, id: span::Id) -> bool {
        self.events.borrow_mut().push(SubscriberEvent::TryClose(id));
        false
    }
}

/// Wrapper representing span tree received from [`TestSubscriber`].
pub struct SubscriberAssert {
    name_to_span: HashMap<span::Id, String>,
    events: Vec<SubscriberEvent>,
}

impl SubscriberAssert {
    /// Checks whether next tracing event is creation of a new span with the
    /// given name and fields.
    pub fn new_span<S: AsSpan + ?Sized>(mut self, expected: &S) -> Self {
        let current_event = self.events.remove(0);
        match current_event {
            SubscriberEvent::NewSpan(span) => {
                assert_eq!(expected.name(), span.metadata.name());
                if expected.is_strict() {
                    assert_eq!(
                        expected.fields().len(),
                        span.fields.len(),
                        "Fields count doesn't match, expected: {:?}, actual: {:?}",
                        expected.fields(),
                        span.fields,
                    )
                }
                expected.fields().into_iter().for_each(|(f_name, val)| {
                    assert_eq!(
                        Some(&val),
                        span.fields.get(&f_name),
                        "Field {} in span {} either doesn't exist or has value \
                         different from {}, values: {:?}",
                        f_name,
                        expected.name(),
                        val,
                        span.fields,
                    );
                });
                if let Some(level) = expected.level() {
                    assert_eq!(
                        &level,
                        span.metadata.level(),
                        "Expected level: '{}' in span: {:?}",
                        level,
                        span,
                    )
                }
                if let Some(target) = expected.target() {
                    assert_eq!(
                        target,
                        span.metadata.target(),
                        "Expected target: '{}' in span: {:?}",
                        target,
                        span,
                    )
                }

                self.name_to_span
                    .insert(span.id, span.metadata.name().to_owned());
            }
            ev => assert!(
                false,
                "Expected `NewSpan`: {}, got {:?}, remaining events: {:?}",
                expected.name(),
                ev,
                self.events,
            ),
        }
        self
    }

    /// Checks whether the next step is entering the span with the given name.
    pub fn enter<S: AsSpan + ?Sized>(mut self, span: &S) -> Self {
        let current_event = self.events.remove(0);
        match current_event {
            SubscriberEvent::Enter(id) => match self.name_to_span.get(&id) {
                None => assert!(
                    false,
                    "No span with id: {:?}, registered spans: {:?}",
                    id, self.name_to_span,
                ),
                Some(actual_name) => assert_eq!(
                    span.name(),
                    actual_name.as_str(),
                    "Entered span with name: {}, expected: {}",
                    actual_name,
                    span.name(),
                ),
            },
            ev => assert!(
                false,
                "Expected `Enter`: {}, got: {:?}, remaining events: {:?}",
                span.name(),
                ev,
                self.events,
            ),
        }
        self.re_enter(span)
    }

    /// Checks whether the next step is exiting the span with the given name.
    pub fn exit<S: AsSpan + ?Sized>(mut self, span: &S) -> Self {
        let current_event = self.events.remove(0);
        match current_event {
            SubscriberEvent::Exit(id) => match self.name_to_span.get(&id) {
                None => assert!(
                    false,
                    "No span with id: {:?}, registered spans: {:?}",
                    id, self.name_to_span,
                ),
                Some(actual_name) => assert_eq!(
                    span.name(),
                    actual_name.as_str(),
                    "Exited span with name: {}, expected: {}",
                    actual_name,
                    span.name(),
                ),
            },
            ev => assert!(
                false,
                "Expected `Exit`: {}, got: {:?}, remaining events: {:?}",
                span.name(),
                ev,
                self.events,
            ),
        }
        self
    }

    /// Checks whether the next step is attempt to close span with the given
    /// name.
    pub fn try_close<S: AsSpan + ?Sized>(mut self, span: &S) -> Self {
        let current_event = self.events.remove(0);
        match current_event {
            SubscriberEvent::TryClose(id) => match self.name_to_span.get(&id) {
                None => assert!(
                    false,
                    "No span with id: {:?}, registered spans: {:?}",
                    id, self.name_to_span,
                ),
                Some(actual_name) => assert_eq!(
                    span.name(),
                    actual_name.as_str(),
                    "Attempted to close span with name: {}, expected: {}",
                    actual_name,
                    span.name(),
                ),
            },
            ev => assert!(
                false,
                "Expected `TryClose`: {}, got: {:?}, remaining events: {:?}",
                span.name(),
                ev,
                self.events,
            ),
        }
        self
    }

    /// Checks whether next step is event with the given level, optionally
    /// target and fields.
    pub fn event(mut self, level: Level, target: Option<&str>, fields: Vec<(&str, &str)>) -> Self {
        let current_event = self.events.remove(0);
        match current_event {
            SubscriberEvent::Event(ev) => {
                assert_eq!(ev.metadata.level(), &level);
                if let Some(target) = target {
                    assert_eq!(ev.metadata.target(), target);
                }
                for (name, value) in fields {
                    assert_eq!(ev.fields.get(name).map(String::as_str), Some(value))
                }
            }
            ev => assert!(
                false,
                "Expected `Event`, got: {:?}, remaining events: {:?}",
                ev, self.events,
            ),
        }
        self
    }

    /// Checks whether next steps are `new_span` then `enter` then `exit` and
    /// finally `try_close`.
    pub fn simple_span<S: AsSpan + ?Sized>(self, span: &S) -> Self {
        self.new_span(span)
            .enter(span)
            .exit(span)
            .re_enter(span)
            .try_close(span)
    }

    /// Checks whether next to steps is creation of a new span with the given
    /// name and entering it.
    pub fn enter_new_span<S: AsSpan + ?Sized>(self, span: &S) -> Self {
        self.new_span(span).enter(span).re_enter(span)
    }

    /// Checks whether next two steps is exiting the span with the given name
    /// and attempt to close it.
    pub fn close_exited<S: AsSpan + ?Sized>(self, span: &S) -> Self {
        self.exit(span).re_enter(span).try_close(span)
    }

    /// Checks whether next two steps is exiting and re-entering the same span
    /// with the given name.
    ///
    /// This may be useful in case of tracing when sync object is resolved in
    /// async context.
    pub fn re_enter<S: AsSpan + ?Sized>(self, span: &S) -> Self {
        use SubscriberEvent as Ev;

        let first = self.events.get(0);
        let second = self.events.get(1);
        match (first, second) {
            (Some(Ev::Exit(first)), Some(Ev::Enter(second))) if first == second => {
                let next_span = self
                    .name_to_span
                    .get(first)
                    .unwrap_or_else(|| panic!("No span with id '{:?}'", first));
                if next_span.as_str() == span.name() {
                    return self.exit(span).enter(span);
                }
            }
            (Some(Ev::Enter(first)), Some(Ev::Exit(second))) if first == second => {
                let next_span = self
                    .name_to_span
                    .get(first)
                    .unwrap_or_else(|| panic!("No span with id '{:?}'", first));
                if next_span.as_str() == span.name() {
                    return self.enter(span).exit(span);
                }
            }
            _ => {}
        }
        self
    }
}

/// Struct that can be compared to span recorded by [`TestSubscriber`].
pub struct SpanLike {
    name: String,
    level: Option<Level>,
    target: Option<String>,
    fields: Vec<(String, String)>,
    strict_fields: bool,
}

/// Abstraction over types that can be compared with span form `tracing` crate.
pub trait AsSpan {
    /// Name of span.
    fn name(&self) -> &str;

    /// [`Level`] of span.
    fn level(&self) -> Option<Level> {
        None
    }

    /// `target` of span.
    fn target(&self) -> Option<&str> {
        None
    }

    /// `fields` recorded within a span.
    fn fields(&self) -> Vec<(String, String)> {
        vec![]
    }

    /// Whether fields should be checked strictly.
    fn is_strict(&self) -> bool {
        false
    }
}

impl AsSpan for str {
    fn name(&self) -> &str {
        self
    }
}

impl AsSpan for SpanLike {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn level(&self) -> Option<Level> {
        self.level
    }

    fn target(&self) -> Option<&str> {
        self.target.as_ref().map(String::as_str)
    }

    fn fields(&self) -> Vec<(String, String)> {
        self.fields.clone()
    }

    fn is_strict(&self) -> bool {
        self.strict_fields
    }
}

/// Extension that allows fluent construction of [`SpanLike`].
pub trait SpanExt {
    /// Sets the given `name`.
    fn with_name(self, name: &str) -> SpanLike;

    /// Sets the given `level`.
    fn with_level(self, level: Level) -> SpanLike;

    /// Sets the given `target`.
    fn with_target(self, target: &str) -> SpanLike;

    /// Adds the given `field` with the `value`.
    fn with_field(self, field: &str, value: &str) -> SpanLike;

    /// Sets `is_strict` to the given value.
    fn with_strict_fields(self, strict: bool) -> SpanLike;
}

impl SpanExt for &str {
    fn with_name(self, name: &str) -> SpanLike {
        SpanLike {
            name: name.to_owned(),
            level: None,
            target: None,
            fields: vec![],
            strict_fields: false,
        }
    }

    fn with_level(self, level: Level) -> SpanLike {
        SpanLike {
            name: self.to_owned(),
            level: Some(level),
            target: None,
            fields: vec![],
            strict_fields: false,
        }
    }

    fn with_target(self, target: &str) -> SpanLike {
        SpanLike {
            name: self.to_owned(),
            level: None,
            target: Some(target.to_owned()),
            fields: vec![],
            strict_fields: false,
        }
    }

    fn with_field(self, field: &str, value: &str) -> SpanLike {
        SpanLike {
            name: self.to_owned(),
            level: None,
            target: None,
            fields: vec![(field.to_owned(), value.to_owned())],
            strict_fields: false,
        }
    }

    fn with_strict_fields(self, strict: bool) -> SpanLike {
        SpanLike {
            name: self.to_owned(),
            level: None,
            target: None,
            fields: vec![],
            strict_fields: strict,
        }
    }
}

impl SpanExt for SpanLike {
    fn with_name(mut self, name: &str) -> SpanLike {
        self.name = name.to_owned();
        self
    }

    fn with_level(mut self, level: Level) -> SpanLike {
        self.level = Some(level);
        self
    }

    fn with_target(mut self, target: &str) -> SpanLike {
        self.target = Some(target.to_owned());
        self
    }

    fn with_field(mut self, field: &str, value: &str) -> SpanLike {
        self.fields.push((field.to_owned(), value.to_owned()));
        self
    }

    fn with_strict_fields(mut self, strict: bool) -> SpanLike {
        self.strict_fields = strict;
        self
    }
}
