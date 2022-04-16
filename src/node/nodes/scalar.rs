/*
 * This Source Code Form is subject to the terms of the
 * Mozilla Public License, v. 2.0. If a copy of the MPL
 * was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use crate::{
    node::{
        graph::Storage,
        nodes::{NodeContext, NodeData, NodeIndex, NodeMark, NodeSpecific, Tag},
    },
    token::{ScalarStyle, Slice},
};

pub(in crate::node) struct ScalarNode<'de>
{
    parent: Option<NodeIndex>,
    id:     NodeIndex,

    scalar: Slice<'de>,
}

impl<'de> ScalarNode<'de>
{
    pub fn root(id: NodeIndex, data: Slice<'de>) -> Self
    {
        Self::with_parent(id, data, None)
    }

    pub fn new(id: NodeIndex, parent: NodeIndex, data: Slice<'de>) -> Self
    {
        Self::with_parent(id, data, Some(parent))
    }

    pub fn root_with_data(data: Slice<'de>) -> impl FnOnce(NodeIndex) -> ScalarNode<'de>
    {
        move |id| Self::root(id, data)
    }

    pub fn new_with_data(
        parent: NodeIndex,
        data: Slice<'de>,
    ) -> impl FnOnce(NodeIndex) -> ScalarNode<'de>
    {
        move |id| Self::new(id, parent, data)
    }

    pub fn id(&self) -> NodeIndex
    {
        self.id
    }

    pub fn parent(&self) -> Option<NodeIndex>
    {
        self.parent
    }

    pub fn data<'a>(&self, g: &'a Storage<'de>) -> ScalarDataRef<'a, 'de>
    {
        let data = &g.node_data()[self.id];

        ScalarDataRef::new(data)
    }

    pub fn data_mut<'a>(&self, g: &'a mut Storage<'de>) -> ScalarDataMut<'a, 'de>
    {
        let data = &mut g.node_data_mut()[self.id];

        ScalarDataMut::new(data)
    }

    fn with_parent(id: NodeIndex, scalar: Slice<'de>, parent: Option<NodeIndex>) -> Self
    {
        Self { parent, id, scalar }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::node) struct ScalarData
{
    style: ScalarStyle,
}

impl ScalarData
{
    pub const fn new(style: ScalarStyle) -> Self
    {
        Self { style }
    }

    pub fn opaque(self) -> NodeSpecific
    {
        self.into()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(in crate::node) struct ScalarDataRef<'a, 'de>
{
    data: &'a NodeData<'de>,
}

impl<'a, 'de> ScalarDataRef<'a, 'de>
{
    fn new(data: &'a NodeData<'de>) -> Self
    {
        Self { data }
    }

    pub const fn anchor(&self) -> Option<&Slice<'de>>
    {
        self.data.anchor.as_ref()
    }

    pub const fn tag(&self) -> Option<&Tag<'de>>
    {
        self.data.tag.as_ref()
    }

    pub const fn context(&self) -> &NodeContext
    {
        &self.data.context
    }

    pub const fn mark(&self) -> &NodeMark
    {
        &self.data.mark
    }

    pub fn style(&self) -> &ScalarStyle
    {
        &self.scalar().style
    }

    fn scalar(&self) -> &ScalarData
    {
        use NodeSpecific::Scalar;

        match self.data.node_specific
        {
            Scalar(ref s) => s,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub(in crate::node) struct ScalarDataMut<'a, 'de>
{
    data: &'a mut NodeData<'de>,
}

impl<'a, 'de> ScalarDataMut<'a, 'de>
{
    fn new(data: &'a mut NodeData<'de>) -> Self
    {
        let mut this = Self { data };
        let _assert = this.scalar();

        this
    }

    pub fn anchor(&mut self) -> Option<&mut Slice<'de>>
    {
        self.data.anchor.as_mut()
    }

    pub fn tag(&mut self) -> Option<&mut Tag<'de>>
    {
        self.data.tag.as_mut()
    }

    pub fn context(&mut self) -> &mut NodeContext
    {
        &mut self.data.context
    }

    pub fn mark(&mut self) -> &mut NodeMark
    {
        &mut self.data.mark
    }

    pub fn style(&mut self) -> &mut ScalarStyle
    {
        &mut self.scalar().style
    }

    fn scalar(&mut self) -> &mut ScalarData
    {
        use NodeSpecific::Scalar;

        match self.data.node_specific
        {
            Scalar(ref mut s) => s,
            _ => unreachable!(),
        }
    }
}
