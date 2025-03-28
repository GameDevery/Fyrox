// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Build context is used to decouple explicit UI state modification. See [`BuildContext`] docs for
//! more info.

use crate::style::resource::StyleResource;
use crate::{
    core::pool::Handle, font::FontResource, message::UiMessage, RestrictionEntry, UiNode,
    UserInterface,
};
use fyrox_graph::BaseSceneGraph;
use std::{
    ops::{Index, IndexMut},
    sync::mpsc::Sender,
};

/// Build context is used to decouple explicit UI state modification. Its main use is in the various widget
/// builders. Internally, it is just a mutable reference to the UI state. UI can be modified (add nodes, clone,
/// link, etc.) via build context. This is needed to explicitly highlight that it used to modify the UI
/// state. It is **not recommended** to use BuildContext for mutable access to widgets at runtime! _Use message
/// passing_ to modify widgets at runtime, otherwise you will easily break invariant (inner state) of widgets.
/// The only place where it's allowed to directly mutate widget's state is at build stage (inside `build`
/// method of your widget builder).
///
/// ## Examples
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     core::{visitor::prelude::*, reflect::prelude::*, type_traits::prelude::*,},
/// #     define_widget_deref,
/// #     message::UiMessage,
/// #     widget::{Widget, WidgetBuilder},
/// #     BuildContext, Control, UiNode, UserInterface,
/// # };
/// # use std::{
/// #     any::{Any, TypeId},
/// #     ops::{Deref, DerefMut},
/// # };
/// # use fyrox_core::uuid_provider;
/// #
/// #[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
/// #[reflect(derived_type = "UiNode")]
/// struct MyWidget {
///     widget: Widget,
/// }
/// #
/// # define_widget_deref!(MyWidget);
/// #
/// # uuid_provider!(MyWidget = "a93ec1b5-e7c8-4919-ac19-687d8c99f6bd");
/// #
/// # impl Control for MyWidget {
/// #     fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
/// #         todo!()
/// #     }
/// # }
///
/// struct MyWidgetBuilder {
///     widget_builder: WidgetBuilder,
/// }
///
/// impl MyWidgetBuilder {
///     pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
///         let my_widget = MyWidget {
///             widget: self.widget_builder.build(ctx),
///         };
///
///         ctx.add_node(UiNode::new(my_widget))
///     }
/// }
/// ```
pub struct BuildContext<'a> {
    ui: &'a mut UserInterface,
    pub style: StyleResource,
}

impl Index<Handle<UiNode>> for BuildContext<'_> {
    type Output = UiNode;

    fn index(&self, index: Handle<UiNode>) -> &Self::Output {
        &self.ui.nodes[index]
    }
}

impl IndexMut<Handle<UiNode>> for BuildContext<'_> {
    fn index_mut(&mut self, index: Handle<UiNode>) -> &mut Self::Output {
        &mut self.ui.nodes[index]
    }
}

impl<'a> From<&'a mut UserInterface> for BuildContext<'a> {
    fn from(ui: &'a mut UserInterface) -> Self {
        Self {
            style: ui.style.clone(),
            ui,
        }
    }
}

impl BuildContext<'_> {
    /// Returns default font instance used by the UI.
    pub fn default_font(&self) -> FontResource {
        self.ui.default_font.clone()
    }

    /// Returns current message sender of the UI, that is used for message passing mechanism. You can
    /// send messages for your widgets inside your builders, however this has limited use and should
    /// be avoided in the favor of explicit state modification to not overload message pipeline.
    pub fn sender(&self) -> Sender<UiMessage> {
        self.ui.sender()
    }

    /// Adds a new widget to the UI. See [`UiNode`] docs for more info, [`UiNode::new`] in particular.
    pub fn add_node(&mut self, node: UiNode) -> Handle<UiNode> {
        self.ui.add_node(node)
    }

    /// Links the child widget with the parent widget. Child widget's position and size will be restricted by
    /// the new parent. When a widget is linked to other widget, its coordinates become relative to it parent.
    pub fn link(&mut self, child: Handle<UiNode>, parent: Handle<UiNode>) {
        self.ui.link_nodes(child, parent, false)
    }

    /// Copies a widget, adds it to the UI, links it to the root node of the UI and returns the handle to it.
    pub fn copy(&mut self, node: Handle<UiNode>) -> Handle<UiNode> {
        self.ui.copy_node(node)
    }

    /// Tries to fetch the node by its handle. Returns `None` if the handle is invalid.
    pub fn try_get_node(&self, node: Handle<UiNode>) -> Option<&UiNode> {
        self.ui.try_get(node)
    }

    /// Tries to fetch the node by its handle. Returns `None` if the handle is invalid.
    pub fn try_get_node_mut(&mut self, node: Handle<UiNode>) -> Option<&mut UiNode> {
        self.ui.nodes.try_borrow_mut(node)
    }

    /// Pushes a new picking restriction to the picking-restriction stack. See [`UserInterface::push_picking_restriction`]
    /// docs for more info.
    pub fn push_picking_restriction(&mut self, restriction: RestrictionEntry) {
        self.ui.push_picking_restriction(restriction)
    }

    /// Explicitly removes picking restriction for the given node from the picking-restriction stack. See
    /// [`UserInterface::remove_picking_restriction`] docs for more info.
    pub fn remove_picking_restriction(&mut self, node: Handle<UiNode>) {
        self.ui.remove_picking_restriction(node)
    }

    /// Returns an immutable reference to the user interface.
    pub fn inner(&self) -> &UserInterface {
        self.ui
    }

    /// Returns a mutable reference to the user interface.
    pub fn inner_mut(&mut self) -> &mut UserInterface {
        self.ui
    }

    /// Sends a message during build stage. It has quite limited use, but could be unavoidable in
    /// for cases when you need to do some action that relies on fully performed layout stage. When a
    /// widget is being built, you can't fetch any layout info of it since it wasn't calculated yet.
    /// In this case all you can do is to "postpone" your action for later moment in current frame
    /// by sending a message.
    pub fn send_message(&self, message: UiMessage) {
        self.ui.send_message(message);
    }
}
