use crate::manifest::ServiceManifest;
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DagError {
    #[error("Dependency cycle detected involving service: {0}")]
    CycleDetected(String),
    #[error("Missing dependency '{0}' required by service '{1}'")]
    MissingDependency(String, String),
}

pub struct ServiceDag {
    manifests: HashMap<String, ServiceManifest>,
}

impl ServiceDag {
    pub fn new(manifests: Vec<ServiceManifest>) -> Result<Self, DagError> {
        let mut map = HashMap::new();
        for m in manifests {
            map.insert(m.name.clone(), m);
        }

        // Validate missing dependencies
        for (name, manifest) in &map {
            for dep in &manifest.dependencies {
                if !map.contains_key(dep) {
                    return Err(DagError::MissingDependency(dep.clone(), name.clone()));
                }
            }
        }

        let dag = Self { manifests: map };
        // Test ordering for cycle check
        dag.topological_sort()?;
        Ok(dag)
    }

    pub fn topological_sort(&self) -> Result<Vec<ServiceManifest>, DagError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for name in self.manifests.keys() {
            in_degree.insert(name.clone(), 0);
            graph.insert(name.clone(), vec![]);
        }

        for (name, manifest) in &self.manifests {
            for dep in &manifest.dependencies {
                graph.get_mut(dep).unwrap().push(name.clone());
                *in_degree.get_mut(name).unwrap() += 1;
            }
        }

        let mut queue: VecDeque<String> = VecDeque::new();
        for (name, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(name.clone());
            }
        }

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop_front() {
            sorted.push(self.manifests.get(&node).unwrap().clone());

            if let Some(dependents) = graph.get(&node) {
                for dep in dependents {
                    let deg = in_degree.get_mut(dep).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        if sorted.len() != self.manifests.len() {
            let unvisited: Vec<String> = in_degree
                .into_iter()
                .filter(|(_, deg)| *deg > 0)
                .map(|(n, _)| n)
                .collect();
            return Err(DagError::CycleDetected(unvisited.join(", ")));
        }

        Ok(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::RestartPolicy;
    use std::path::PathBuf;

    fn mock_manifest(name: &str, deps: Vec<&str>) -> ServiceManifest {
        ServiceManifest {
            name: name.to_string(),
            description: format!("Mock service {}", name),
            binary_path: PathBuf::from("/bin/mock"),
            args: vec![],
            env: vec![],
            dependencies: deps.into_iter().map(String::from).collect(),
            priority: 1,
            restart_policy: RestartPolicy::Always,
            health_check: Default::default(),
        }
    }

    #[test]
    fn test_valid_dag_ordering() {
        let bus = mock_manifest("bus", vec![]);
        let logging = mock_manifest("logging", vec!["bus"]);
        let process = mock_manifest("process", vec!["bus", "logging"]);

        let dag = ServiceDag::new(vec![process, logging, bus]).unwrap();
        let sorted = dag.topological_sort().unwrap();

        let names: Vec<String> = sorted.into_iter().map(|m| m.name).collect();
        assert_eq!(names[0], "bus");
        assert_eq!(names[1], "logging");
        assert_eq!(names[2], "process");
    }

    #[test]
    fn test_cycle_detection() {
        let a = mock_manifest("service_a", vec!["service_b"]);
        let b = mock_manifest("service_b", vec!["service_a"]);

        let res = ServiceDag::new(vec![a, b]);
        assert!(res.is_err());
        match res.err().unwrap() {
            DagError::CycleDetected(_) => {}
            _ => panic!("Expected CycleDetected error"),
        }
    }
}
