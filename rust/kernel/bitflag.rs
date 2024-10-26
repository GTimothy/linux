// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2024 Google LLC.

//! A constrained Bitflag wrapper with its associated builder implementation as a macro.

#[doc(inline)]
pub use crate::macros::bitflag;

/// A marker type to use as part of the BitflagBuilder typestate. It indicates that part of the build configuration is missing.
#[derive(Debug)]
pub struct Missing<Part> {
    t: core::marker::PhantomData<Part>,
}

/// A marker type to use as part of the BitflagBuilder typestate. It indicates that part of the build configuration is valid.
#[derive(Debug)]
pub struct Valid<Part> {
    t: core::marker::PhantomData<Part>,
}
