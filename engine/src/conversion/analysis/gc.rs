// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::{HashMap, HashSet};

use autocxx_parser::TypeDatabase;

use crate::{
    conversion::api::{Api, ApiAnalysis},
    types::TypeName,
};

/// This is essentially mark-and-sweep garbage collection of the
/// Apis that we've discovered. Why do we do this, you might wonder?
/// It seems a bit strange given that we pass an explicit allowlist
/// to bindgen.
/// There are two circumstances under which we want to discard
/// some of the APIs we encounter parsing the bindgen.
/// 1) We simplify some struct to be non-POD. In this case, we'll
///    discard all the fields within it. Those fields can be, and
///    in fact often _are_, stuff which we have trouble converting
///    e.g. std::string or std::string::value_type or
///    my_derived_thing<std::basic_string::value_type> or some
///    other permutation. In such cases, we want to discard those
///    field types with prejudice.
/// 2) block! may be used to ban certain APIs. This often eliminates
///    some methods from a given struct/class. In which case, we
///    don't care about the other parameter types passed into those
///    APIs either.
pub(crate) fn filter_apis_by_following_edges_from_allowlist<T: ApiAnalysis>(
    mut apis: Vec<Api<T>>,
    type_database: &TypeDatabase,
) -> Vec<Api<T>> {
    let mut todos: Vec<_> = apis
        .iter()
        .filter(|api| {
            let tnforal = api.typename_for_allowlist();
            type_database.is_on_allowlist(&tnforal.to_cpp_name())
        })
        .map(Api::typename)
        .collect();
    let mut by_typename: HashMap<TypeName, Vec<Api<T>>> = HashMap::new();
    for api in apis.drain(..) {
        let tn = api.typename();
        by_typename.entry(tn).or_default().push(api);
    }
    let mut done = HashSet::new();
    let mut output = Vec::new();
    while !todos.is_empty() {
        let todo = todos.remove(0);
        if done.contains(&todo) {
            continue;
        }
        if let Some(mut these_apis) = by_typename.remove(&todo) {
            todos.extend(these_apis.iter().flat_map(|api| api.deps.iter().cloned()));
            output.append(&mut these_apis);
        } // otherwise, probably an intrinsic e.g. uint32_t.
        done.insert(todo);
    }
    output
}
