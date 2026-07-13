import os
import re

os.makedirs('src/transform', exist_ok=True)

with open('src/transform.rs', 'r') as f:
    content = f.read()

# We will use regex to extract the parts.
# Let's extract tests out first to avoid confusion
tests_split = content.split('#[cfg(test)]\nmod tests {')
main_content = tests_split[0]
tests_content = '#[cfg(test)]\nmod tests {' + tests_split[1] if len(tests_split) > 1 else ''

mod_rs = """//! Stat transforms module.
//!
//! Transforms modify stat values after sources are collected.
//! Transforms can read other stats (dependencies) and must declare
//! them explicitly via `depends_on()`.

pub mod core;
pub mod standard;
pub mod conditional;

pub use core::*;
pub use standard::*;
pub use conditional::*;

""" + tests_content.replace('use super::*;', 'use super::*;\n    use rustc_hash::FxHashMap;')

# core.rs
core_rs = """use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::{StatNumeric, StatValue};
use crate::stat_id::StatId;
use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};

""" + main_content[main_content.find('/// Phase for transform'):main_content.find('/// A multiplicative transform')]

# standard.rs
standard_rs = """use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::{StatNumeric, StatValue};
use crate::stat_id::StatId;
use crate::transform::core::{TransformPhase, StackRule, StatTransform, ClampBounds};
use rustc_hash::FxHashMap;

""" + main_content[main_content.find('/// A multiplicative transform'):main_content.find('/// A conditional transform')] + main_content[main_content.find('/// A transform that scales based on another stat.'):]

# conditional.rs
conditional_rs = """use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::{StatNumeric, StatValue};
use crate::stat_id::StatId;
use crate::transform::core::{TransformPhase, StackRule, StatTransform};
use rustc_hash::FxHashMap;

""" + main_content[main_content.find('/// A conditional transform'):main_content.find('/// A transform that scales based on another stat.')]

with open('src/transform/mod.rs', 'w') as f:
    f.write(mod_rs)

with open('src/transform/core.rs', 'w') as f:
    f.write(core_rs)

with open('src/transform/standard.rs', 'w') as f:
    f.write(standard_rs)

with open('src/transform/conditional.rs', 'w') as f:
    f.write(conditional_rs)

os.remove('src/transform.rs')
