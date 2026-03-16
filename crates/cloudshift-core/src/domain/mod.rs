//! Domain layer — the core of CloudShift.
//!
//! Contains entities, value objects, domain events, port traits (interfaces),
//! and domain services. This module has ZERO dependencies on infrastructure,
//! frameworks, or I/O. All types are immutable by default.

pub mod entities;
pub mod events;
pub mod ports;
pub mod services;
pub mod value_objects;
