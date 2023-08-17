use crate::component::Component::{CurrentSrc, Resistor, VoltageSrc};
use crate::container::Container;
use crate::solvers::solver::{Solver, Step};
use crate::util::PrettyPrint;
use ndarray::{s, ArrayBase, Ix2, OwnedRepr};
use operations::math::{matrix_to_latex, EquationRepr};
use operations::prelude::{Divide, Negate, Operation, Sum, Text, Value, Variable};
use std::cell::RefCell;
use std::rc::Rc;

pub struct NodeSolver {
    container: Rc<RefCell<Container>>,
    a_matrix: ndarray::Array2<Operation>,
    x_matrix: ndarray::Array2<Operation>,
    z_matrix: ndarray::Array2<Operation>,
}

impl Solver for NodeSolver {
    fn new(container: Rc<RefCell<Container>>) -> NodeSolver {
        container.borrow_mut().create_nodes();
        let n = container.borrow().nodes().len(); // Node Count
        let m = container // Source Count
            .borrow()
            .get_elements()
            .iter()
            .fold(0, |acc: usize, x| match x.class {
                VoltageSrc | CurrentSrc => acc + 1,
                _ => acc,
            });

        // https://lpsa.swarthmore.edu/Systems/Electrical/mna/MNA3.html#B_matrix
        NodeSolver {
            container: container.clone(),
            a_matrix: form_a_matrix(container.clone(), n, m),
            x_matrix: form_x_matrix(container.clone(), n, m),
            z_matrix: form_z_matrix(container.clone(), n, m),
        }
    }

    /// Returns a string that represents the matrix equation to solve the circuit.
    fn solve(&self) -> Result<Vec<Step>, String> {
        let mut steps: Vec<Step> = Vec::new();
        let inverse_a_matrix: ndarray::Array2<Operation> = self.a_matrix.clone();
        // solve::inverse(&mut inverse_a_matrix).unwrap();
        // Wrap in matrix
        // [x] = [A]^-1 * [z]

        steps.push(Step {
            label: "A Matrix".to_string(),
            sub_steps: Some(vec![Variable(Rc::new(self.a_matrix.clone()))]),
        });

        steps.push(Step {
            label: "Z Matrix".to_string(),
            sub_steps: Some(vec![Variable(Rc::new(self.z_matrix.clone()))]),
        });

        steps.push(Step {
            label: "X Matrix".to_string(),
            sub_steps: Some(vec![Variable(Rc::new(self.x_matrix.clone()))]),
        });

        steps.push(Step {
            label: "Inverse A Matrix".to_string(),
            sub_steps: Some(vec![Text("TODO".to_string())]),
        });

        steps.push(Step {
            label: "Final Equation".to_string(),
            sub_steps: Some(vec![]),
        });

        steps.push(Step {
            label: "Final Equation".to_string(),
            sub_steps: Some(vec![Text(format!(
                "{} = {}^{{-1}} * {}",
                matrix_to_latex(self.x_matrix.clone()),
                matrix_to_latex(inverse_a_matrix),
                matrix_to_latex(self.z_matrix.clone())
            ))]),
        });

        Ok(steps)
    }
}

fn form_a_matrix(
    container: Rc<RefCell<Container>>,
    n: usize,
    m: usize,
) -> ndarray::Array2<Operation> {
    let mut matrix: ArrayBase<OwnedRepr<Operation>, Ix2> =
        ndarray::Array2::<Operation>::zeros((n + m, n + m));

    let g: ndarray::Array2<Operation> = form_g_matrix(container.clone(), n);
    let b: ndarray::Array2<Operation> = form_b_matrix(container.clone(), n, m);
    let c: ndarray::Array2<Operation> = form_c_matrix(container.clone(), n, m);
    let d: ndarray::Array2<Operation> = form_d_matrix(container.clone(), m);

    matrix.slice_mut(s![0..n, 0..n]).assign(&g);
    matrix.slice_mut(s![0..n, n..n + m]).assign(&b);
    matrix.slice_mut(s![n..n + m, 0..n]).assign(&c);
    matrix.slice_mut(s![n..n + m, n..n + m]).assign(&d);

    matrix
}

fn form_g_matrix(container: Rc<RefCell<Container>>, n: usize) -> ndarray::Array2<Operation> {
    let mut matrix: ArrayBase<OwnedRepr<Operation>, Ix2> =
        ndarray::Array2::<Operation>::zeros((n, n));
    let mut nodes = container.borrow().nodes().clone();
    let _elements = container.borrow().get_elements().clone();

    nodes.sort_by(|a, b| a.upgrade().unwrap().id.cmp(&b.upgrade().unwrap().id));

    assert_eq!(nodes.len(), n);

    // Form the diagonal
    for (i, tool) in nodes.iter().enumerate() {
        let equation_members: Vec<EquationRepr> = tool
            .upgrade()
            .unwrap()
            .members
            .iter()
            .filter(|x| x.upgrade().unwrap().class == Resistor)
            .map(|x| EquationRepr::from(x.upgrade().unwrap()))
            .collect();
        let set: Vec<Operation> = equation_members
            .into_iter()
            .map(|x| {
                Divide(
                    Some(Box::new(Value(1.0))),
                    Some(Box::new(Variable(Rc::new(x)))),
                )
            })
            .collect();

        matrix[[n - i - 1, n - i - 1]] = Sum(set);
    }

    // Form the off-diagonal
    // Find all resistors between two nodes
    for (i, tool) in nodes.iter().enumerate() {
        for (j, tool2) in nodes.iter().enumerate() {
            if i == j {
                continue;
            }
            let mut set: Vec<Operation> = Vec::new();
            for element in &tool.upgrade().unwrap().members {
                let element = element.upgrade().unwrap();
                if element.class != Resistor {
                    continue;
                }
                for element2 in tool2.upgrade().unwrap().members.clone() {
                    let element2 = element2.upgrade().unwrap();
                    if element2.class != Resistor {
                        continue;
                    }
                    if element.id == element2.id {
                        set.push(Negate(Some(Box::new(Divide(
                            Some(Box::new(Value(1.0))),
                            Some(Box::from(Variable(element.clone()))),
                        )))));
                    }
                }
            }
            matrix[[n - i - 1, n - j - 1]] = Sum(set);
        }
    }
    matrix
}

fn form_b_matrix(
    container: Rc<RefCell<Container>>,
    n: usize,
    m: usize,
) -> ndarray::Array2<Operation> {
    let mut matrix: ArrayBase<OwnedRepr<Operation>, Ix2> =
        ndarray::Array2::<Operation>::zeros((n, m));

    for (i, tool) in container.borrow().nodes().iter().enumerate() {
        for (j, element) in container.borrow().get_voltage_sources().iter().enumerate() {
            if tool.upgrade().unwrap().contains(element) {
                if element
                    .upgrade()
                    .unwrap()
                    .positive
                    .contains(&tool.upgrade().unwrap().members[0].upgrade().unwrap().id)
                {
                    matrix[[n - i - 1, j]] = Value(-1.0);
                } else {
                    matrix[[n - i - 1, j]] = Value(1.0);
                }
            }
        }
    }

    matrix
}

fn form_c_matrix(
    container: Rc<RefCell<Container>>,
    n: usize,
    m: usize,
) -> ndarray::Array2<Operation> {
    let mut matrix = form_b_matrix(container.clone(), n, m);
    matrix.swap_axes(0, 1);
    matrix
}

fn form_d_matrix(_container: Rc<RefCell<Container>>, m: usize) -> ndarray::Array2<Operation> {
    let matrix: ArrayBase<OwnedRepr<Operation>, Ix2> = ndarray::Array2::<Operation>::zeros((m, m));
    matrix
}

fn form_z_matrix(
    container: Rc<RefCell<Container>>,
    n: usize,
    m: usize,
) -> ndarray::Array2<Operation> {
    let mut matrix: ArrayBase<OwnedRepr<Operation>, Ix2> =
        ndarray::Array2::<Operation>::zeros((n + m, 1));

    // I Matrix
    // The balance of current flowing in the node.
    for (i, tool) in container.borrow().nodes().iter().enumerate() {
        let mut set: Vec<Operation> = Vec::new();
        for element in &tool.upgrade().unwrap().members {
            let element = element.upgrade().unwrap();
            if element.class != CurrentSrc {
                continue;
            }
            set.push(Value(element.value));
        }
        if set.len() == 0 {
            continue;
        }
        matrix[[i, 0]] = Sum(set);
    }

    // E Matrix
    // The value of the voltage source.
    for (i, source) in container.borrow().get_voltage_sources().iter().enumerate() {
        matrix[[n + i, 0]] = Value(source.upgrade().unwrap().value);
    }

    matrix
}

fn form_x_matrix(
    container: Rc<RefCell<Container>>,
    n: usize,
    m: usize,
) -> ndarray::Array2<Operation> {
    let mut matrix: ArrayBase<OwnedRepr<Operation>, Ix2> =
        ndarray::Array2::<Operation>::zeros((n + m, 1));

    // V Matrix
    for (i, tool) in container.borrow().nodes().iter().enumerate() {
        matrix[[i, 0]] = Variable(Rc::new(EquationRepr::new(
            format!("{}", tool.upgrade().unwrap().pretty_string()),
            0.0,
        )));
    }

    // J Matrix
    for (i, source) in container.borrow().get_voltage_sources().iter().enumerate() {
        matrix[[n + i, 0]] = Variable(Rc::new(EquationRepr::new(
            format!("{}", source.upgrade().unwrap().pretty_string()),
            0.0,
        )));
    }

    matrix
}

#[cfg(test)]
mod tests {
    use crate::solvers::node_matrix_solver::{
        form_b_matrix, form_c_matrix, form_d_matrix, form_g_matrix, NodeSolver,
    };
    use crate::solvers::solver::Solver;
    use crate::util::create_mna_container;
    use ndarray::array;
    use operations::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_node_solver() {
        let mut c = create_mna_container();
        c.create_nodes();
        let _solver: NodeSolver = Solver::new(Rc::new(RefCell::new(c)));
    }

    #[test]
    fn test_a_matrix() {
        let expected = array![
            ["1/R1", "", "", "-1", "0"],
            ["", "1/R2 + 1/R3", "-1/R2", "1", "0"],
            ["", "-1/R2", "1/R2", "0", "1"],
            ["-1", "1", "0", "0", "0"],
            ["0", "0", "1", "0", "0"]
        ];

        let mut c = create_mna_container();
        c.create_nodes();
        let solver: NodeSolver = Solver::new(Rc::new(RefCell::new(c)));

        assert_eq!(solver.a_matrix.map(|x| x.equation_repr()), expected);
    }

    #[test]
    fn test_g_matrix() {
        let expected = array![
            ["1/R1", "", ""],
            ["", "1/R2 + 1/R3", "-1/R2"],
            ["", "-1/R2", "1/R2"]
        ];

        let mut c = create_mna_container();
        c.create_nodes();
        let n = c.nodes().len();

        assert_eq!(
            form_g_matrix(Rc::new(RefCell::new(c)), n).map(|x| x.equation_repr()),
            expected
        );
    }

    #[test]
    fn test_b_matrix() {
        let expected = array![["-1", "0"], ["1", "0"], ["0", "1"]];

        let mut c = create_mna_container();
        c.create_nodes();
        let n = c.nodes().len();
        let m = c.get_voltage_sources().len();

        assert_eq!(
            form_b_matrix(Rc::new(RefCell::new(c)), n, m).map(|x| x.equation_repr()),
            expected
        );
    }

    #[test]
    fn test_c_matrix() {
        let expected = array![["-1", "1", "0"], ["0", "0", "1"]];

        let mut c = create_mna_container();
        c.create_nodes();
        let n = c.nodes().len();
        let m = c.get_voltage_sources().len();

        assert_eq!(
            form_c_matrix(Rc::new(RefCell::new(c)), n, m).map(|x| x.equation_repr()),
            expected
        );
    }

    #[test]
    fn test_d_matrix() {
        let expected = array![["0", "0"], ["0", "0"]];

        let mut c = create_mna_container();
        c.create_nodes();
        let _n = c.nodes().len();
        let m = c.get_voltage_sources().len();

        assert_eq!(
            form_d_matrix(Rc::new(RefCell::new(c)), m).map(|x| x.equation_repr()),
            expected
        );
    }

    #[test]
    fn test_x_matrix() {
        let expected = array![
            ["Node: 1"],
            ["Node: 2"],
            ["Node: 3"],
            ["SRC(V)4: 32 V"],
            ["SRC(V)5: 20 V"]
        ];

        let mut c = create_mna_container();
        c.create_nodes();
        let solver: NodeSolver = Solver::new(Rc::new(RefCell::new(c)));

        assert_eq!(solver.x_matrix.map(|x| x.equation_repr()), expected);
    }

    #[test]
    fn test_z_matrix() {
        let expected = array![["0"], ["0"], ["0"], ["32"], ["20"]];

        let mut c = create_mna_container();
        c.create_nodes();
        let solver: NodeSolver = Solver::new(Rc::new(RefCell::new(c)));

        assert_eq!(solver.z_matrix.map(|x| x.equation_repr()), expected);
    }
}
