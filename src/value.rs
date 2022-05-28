#![allow(missing_docs)]

use std::{marker::PhantomData, sync::Arc};

use crate::{error::Result, reader::Read};

pub struct Yaml<Ty = Eager>
{
    inner: Option<Arc<Node>>,
    _mkr:  PhantomData<Ty>,
}

impl Yaml<Eager>
{
    pub fn load<R>(src: R) -> Result<Self>
    where
        R: Read,
    {
        todo!()
    }

    /// Fetch a scalar datum from the given .path
    ///
    /// May return an empty scalar (`len==0`,`style=Plain`)
    /// if the scalar does not exist
    pub fn scalar<P>(&self, path: P) -> Scalar<'_>
    where
        P: AsPath,
    {
        todo!()
    }

    /// Fetch a scalar datum from the given .path
    ///
    /// May return `None` if the scalar does not exist
    pub fn get_scalar<P>(&self, path: P) -> Option<Scalar<'_>>
    where
        P: AsPath,
    {
        todo!()
    }

    /// Fetch a scalar datum from the given .path
    ///
    /// May return an error explaining what went wrong
    pub fn try_scalar<P>(&self, path: P) -> Result<Scalar<'_>>
    where
        P: AsPath,
    {
        todo!()
    }

    /// Create a new [`Yaml`] view using the node at .path
    /// as the new root
    ///
    /// The returned view may be empty if the node does not
    /// exist
    pub fn node<P>(&self, path: P) -> Yaml
    where
        P: AsPath,
    {
        todo!()
    }

    /*
     * use YARY::path as p;
     * let yaml = ...;
     * let vec = yaml.node(~["name", 5, "name"]);
     *
     * vec.scalar(1)
     */

    /// Create a new [`Yaml`] view using the node at .path
    /// as the new root
    ///
    /// May return `None` if the node does not exist
    pub fn get_node<P>(&self, path: P) -> Option<Yaml>
    where
        P: AsPath,
    {
        todo!()
    }

    /// Create a new [`Yaml`] view using the node at .path
    /// as the new root
    ///
    /// May return an error explaining what went wrong
    pub fn try_node<P>(&self, path: P) -> Result<Yaml>
    where
        P: AsPath,
    {
        todo!()
    }
}

impl Yaml<Lazy>
{
    pub fn lazy<R>(src: R) -> Self
    where
        R: Read,
    {
        todo!()
    }

    pub fn load(self) -> Result<Yaml>
    {
        todo!()
    }

    pub fn node_at<P>(&mut self, path: P) -> Result<NodeRef<'_>>
    where
        P: AsPath,
    {
        todo!()
    }

    pub fn touch<P>(&mut self, path: P) -> Result<bool>
    where
        P: AsPath,
    {
        todo!()
    }

    pub fn freeze(&mut self) -> Yaml
    {
        todo!()
    }

    pub fn freeze_at<P>(&mut self, path: P) -> Result<Yaml>
    where
        P: AsPath,
    {
        todo!()
    }
}

pub struct Scalar<'a>
{
    s: &'a str,
}

pub struct NodeRef<'a>
{
    _mkr: &'a str,
}

pub trait AsPath {}

struct Node;

pub struct Eager;
pub struct Lazy;
