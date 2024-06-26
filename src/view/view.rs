// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashSet,
    sync::{mpsc::SyncSender, Arc},
};

use futures_task::{ArcWake, Waker};

use xilem_core::{Id, IdPath};

use crate::widget::{tree_structure::TreeStructure, AnyWidget, ChangeFlags, Pod, Widget};

xilem_core::generate_view_trait! {View, Widget, Cx, ChangeFlags; : Send}
xilem_core::generate_viewsequence_trait! {ViewSequence, View, ViewMarker, ElementsSplice, Widget, Cx, ChangeFlags, Pod; : Send}
xilem_core::generate_anyview_trait! {AnyView, View, ViewMarker, Cx, ChangeFlags, AnyWidget, BoxedView; + Send}
xilem_core::generate_memoize_view! {Memoize, MemoizeState, View, ViewMarker, Cx, ChangeFlags, s, memoize; + Send}
xilem_core::generate_adapt_view! {View, Cx, ChangeFlags; + Send}
xilem_core::generate_adapt_state_view! {View, Cx, ChangeFlags; + Send}

#[derive(Clone)]
pub struct Cx {
    id_path: IdPath,
    element_id_path: Vec<crate::id::Id>, // Note that this is the widget id type.
    req_chan: SyncSender<IdPath>,
    pub(crate) tree_structure: TreeStructure,
    pub(crate) pending_async: HashSet<Id>,
}

struct MyWaker {
    id_path: IdPath,
    req_chan: SyncSender<IdPath>,
}

impl ArcWake for MyWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        //println!("path = {:?}", arc_self.id_path);
        let _ = arc_self.req_chan.send(arc_self.id_path.clone());
    }
}

impl Cx {
    pub(crate) fn new(req_chan: &SyncSender<IdPath>) -> Self {
        Cx {
            id_path: Vec::new(),
            element_id_path: Vec::new(),
            req_chan: req_chan.clone(),
            pending_async: HashSet::new(),
            tree_structure: TreeStructure::default(),
        }
    }

    pub fn push(&mut self, id: Id) {
        self.id_path.push(id);
    }

    pub fn pop(&mut self) {
        self.id_path.pop();
    }

    pub fn id_path_is_empty(&self) -> bool {
        self.id_path.is_empty()
    }

    pub fn id_path(&self) -> &IdPath {
        &self.id_path
    }

    pub fn element_id_path_is_empty(&self) -> bool {
        self.element_id_path.is_empty()
    }

    /// Return the element id of the current element/widget
    pub fn element_id(&self) -> crate::id::Id {
        *self
            .element_id_path
            .last()
            .expect("element_id path imbalance, there should be an element id")
    }

    /// Run some logic with an id added to the id path.
    ///
    /// This is an ergonomic helper that ensures proper nesting of the id path.
    pub fn with_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, id: Id, f: F) -> T {
        self.push(id);
        let result = f(self);
        self.pop();
        result
    }

    /// Allocate a new id and run logic with the new id added to the id path.
    ///
    /// Also an ergonomic helper.
    pub fn with_new_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, f: F) -> (Id, T) {
        let id = Id::next();
        self.push(id);
        let result = f(self);
        self.pop();
        (id, result)
    }

    /// Run some logic within a new Pod context and return the newly created Pod,
    ///
    /// This logic is usually `View::build` to wrap the returned element into a Pod.
    pub fn with_new_pod<S, E, F>(&mut self, f: F) -> (Id, S, Pod)
    where
        E: Widget + 'static,
        F: FnOnce(&mut Cx) -> (Id, S, E),
    {
        let pod_id = crate::id::Id::next();
        self.element_id_path.push(pod_id);
        let (id, state, element) = f(self);
        self.element_id_path.pop();
        (id, state, Pod::new(element, pod_id))
    }

    /// Run some logic within the context of a given Pod,
    ///
    /// This logic is usually `View::rebuild`
    ///
    /// # Panics
    ///
    /// When the element type `E` is not the same type as the inner `Widget` of the `Pod`.
    pub fn with_pod<T, E, F>(&mut self, pod: &mut Pod, f: F) -> T
    where
        E: Widget + 'static,
        F: FnOnce(&mut E, &mut Cx) -> T,
    {
        self.element_id_path.push(pod.id());
        let element = pod
            .downcast_mut()
            .expect("Element type has changed, this should never happen!");
        let result = f(element, self);
        self.element_id_path.pop();
        result
    }

    pub fn waker(&self) -> Waker {
        futures_task::waker(Arc::new(MyWaker {
            id_path: self.id_path.clone(),
            req_chan: self.req_chan.clone(),
        }))
    }

    /// Add an id for a pending async future.
    ///
    /// Rendering may be delayed when there are pending async futures, to avoid
    /// flashing, and continues when all futures complete, or a timeout, whichever
    /// is first.
    pub fn add_pending_async(&mut self, id: Id) {
        self.pending_async.insert(id);
    }
}
