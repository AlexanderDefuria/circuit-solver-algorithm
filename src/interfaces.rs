use crate::container::Container;
use crate::elements::Element;
use crate::solvers::node_matrix_solver::NodeMatrixSolver;
use crate::solvers::node_step_solver::NodeStepSolver;
use crate::solvers::solver::{serialize_steps, Solver, Step};
use crate::util::{
    create_basic_container, create_basic_supermesh_container, create_basic_supernode_container,
    create_mna_container, create_mna_container_2,
};
use crate::validation::{StatusError, Validation};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize)]
pub struct ContainerSetup {
    pub elements: Vec<Element>,
}

/// This can be used as a test to see if the container is being loaded in properly.
#[wasm_bindgen]
pub fn load_wasm_container(js: JsValue) -> Result<String, StatusError> {
    // This JsValue is a ContainerInterface and also needs operations
    let setup: ContainerSetup = from_value(js).unwrap();
    let container = Container::from(setup);
    container.validate()?;
    Ok(String::from("Loaded Successfully"))
}

#[wasm_bindgen]
pub fn get_tools(container_js: JsValue) -> Result<String, StatusError> {
    let setup: ContainerSetup = from_value(container_js).unwrap();
    let mut c: Container = Container::from(setup);
    c.validate()?;
    c.create_nodes()?;
    c.create_super_nodes();
    let nodes: Vec<Vec<usize>> = c
        .nodes()
        .iter()
        .map(|x| x.upgrade().unwrap().borrow().member_ids())
        .collect();

    Ok(serde_json::to_string(&nodes).unwrap())
}

#[wasm_bindgen]
pub fn solve(matrix: bool, nodal: bool, container_js: JsValue) -> Result<String, JsValue> {
    let setup: ContainerSetup = from_value(container_js)?;
    let mut c: Container = Container::from(setup);
    c.validate()?;

    return match nodal {
        true => {
            c.create_nodes()?;
            c.create_super_nodes();
            let steps: Vec<Step>;
            if matrix {
                let mut solver: NodeMatrixSolver = Solver::new(Rc::new(RefCell::new(c)));
                steps = solver.solve()?;
            } else {
                let mut solver: NodeStepSolver = Solver::new(Rc::new(RefCell::new(c)));
                steps = solver.solve()?;
            }
            serialize_steps(steps)
        }
        false => {
            c.create_meshes();
            c.create_super_meshes();
            Err(JsValue::from(format!(
                "{} Solver not implemented for meshes",
                if matrix { "Matrix" } else { "Step" }
            )))
        }
    };
}

#[wasm_bindgen]
pub fn return_solved_step_example() -> Result<String, JsValue> {
    let mut c: Container = create_mna_container();
    c.create_nodes()?;
    c.create_super_nodes();
    let mut solver: NodeStepSolver = Solver::new(Rc::new(RefCell::new(c)));
    serialize_steps(solver.solve()?)
}

#[wasm_bindgen]
pub fn return_solved_matrix_example() -> Result<String, JsValue> {
    let mut c: Container = create_mna_container();
    c.create_nodes()?;
    c.create_super_nodes();
    let mut solver: NodeMatrixSolver = Solver::new(Rc::new(RefCell::new(c)));
    serialize_steps(solver.solve()?)
}

#[wasm_bindgen]
pub fn test_wasm() -> String {
    "Hello from Rust! 🦀🦀🦀".to_string()
}

#[wasm_bindgen]
pub fn test_error() -> Result<String, JsValue> {
    Err(JsValue::from_str("Error from Rust! 🦀🦀🦀"))
}

#[wasm_bindgen]
pub fn solve_test_container(container_id: i32) -> Result<String, JsValue> {
    let c: Container = match container_id {
        0 => create_basic_container(),
        1 => create_basic_supernode_container(),
        2 => create_basic_supermesh_container(),
        3 => create_mna_container(),
        4 => create_mna_container_2(),
        _ => create_basic_container(),
    };
    let mut solver: NodeStepSolver = Solver::new(Rc::new(RefCell::new(c)));
    serialize_steps(solver.solve()?)
}

impl From<Vec<Element>> for Container {
    fn from(wasm: Vec<Element>) -> Container {
        let mut container = Container::new();
        for element in wasm {
            container.add_element_core(element);
        }
        container
    }
}

impl From<ContainerSetup> for Container {
    fn from(setup: ContainerSetup) -> Container {
        let mut container = Container::new();
        for element in setup.elements {
            container.add_element_core(element);
        }
        container
    }
}
