#![doc = include_str!("../README.md")]

//! # Getting Started
//! Griphos is a network modelling and power flow solver (with OPF to come) that is written in rust, with additional
//! python bindings.
//!
//! Documentation is WIP. Start by taking a look at the documentation for the following modules:
//! - [`mod@network`]: The main network modelling module.
//! - [`mod@foundation`]: General purpose code that is used throughout griphos.
//!
//! For specific network elements, [`mod@generic_element`] show all possible generic functionality of elements. You can
//! then look at the following:
//! - [`mod@line`]: Lines and cables.
//! - [`mod@generator`]: Generation sources with optional voltage control.
//! - [`mod@load`]: ZIP load model for loads and uncontrolled generation.
//! - [`mod@transformer`]: Varous transformers.
//! - [`mod@connector`]: Switches and short circuits.
//! - [`mod@grounding`]: Solid grounding of nodes.
//!
//! Also look at the executables
//! [`run_ejson_network`](../src/run_ejson_network/run_ejson_network.rs.html) source and
//! [`run_test_network`](../src/run_test_network/run_test_network.rs.html) source for simple examples of using griphos
//! to solve power flow.

#[macro_use]
pub mod foundation;
mod graph;
#[macro_use]
pub mod network;
pub mod connector;
pub mod generator;
pub mod generic_element;
pub mod grounding;
pub(crate) mod klu;
pub mod line;
pub mod load;
pub mod transformer;
pub mod network_solvers;
pub mod nodal_rect_solver;
#[allow(unused)]
mod e_json_generated_types;
#[allow(unused)]
pub mod e_json;
pub mod test_networks;
pub mod test_utils;

pub use griphos_proc_macros::TypeStrOf;

#[doc(inline)]
pub use foundation::*;
#[doc(inline)]
pub use network::*;
#[doc(inline)]
pub use connector::*;
#[doc(inline)]
pub use generator::*;
#[doc(inline)]
pub use generic_element::*;
#[doc(inline)]
pub use graph::*;
#[doc(inline)]
pub use grounding::*;
#[doc(inline)]
pub use line::*;
#[doc(inline)]
pub use load::*;
#[doc(inline)]
pub use network_solvers::*;
#[doc(inline)]
pub use test_networks::*;
#[doc(inline)]
pub use transformer::*;
#[doc(inline)]
pub use e_json::*;

#[cfg(has_griphos_extras)]
#[path = "../griphos_extras/src/mod.rs"]
pub mod griphos_extras;

#[cfg(has_griphos_extras)]
#[doc(inline)]
pub use griphos_extras::*;
