use crate::aur::Package;

use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;

type DependencyGraph = HashMap<String, Vec<String>>;

// Recursively resolves dependencies for a list of package names
// see https://en.wikipedia.org/wiki/Dependency_graph
pub(crate) fn build_dependency_graph(
    targets: &[String],
    aur_map: &HashMap<String, Package>,
) -> Result<DependencyGraph> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    // Initialize queue with targets
    for target in targets {
        if aur_map.contains_key(target) {
            queue.push_back(target.clone());
            visited.insert(target.clone());
        }
    }

    let mut graph: DependencyGraph = HashMap::new();
    while let Some(current_pkg_name) = queue.pop_front() {
        let pkg = aur_map.get(&current_pkg_name).ok_or(anyhow::anyhow!("non-AUR package"))?;

        // build up only run-time and build-time deps
        let mut aur_deps = vec![];
        for dep_str in pkg.depends.iter().chain(pkg.make_depends.iter()) {
            // NOTE: meta dependencies will be just skipped sadly
            let clean_name = clean_dep_name(dep_str);
            if aur_map.contains_key(&clean_name) {
                aur_deps.push(clean_name.clone());

                // add to queue for later check
                if !visited.contains(&clean_name) {
                    visited.insert(clean_name.clone());
                    queue.push_back(clean_name);
                }
            }
        }

        graph.insert(current_pkg_name, aur_deps);
    }

    Ok(graph)
}

// Provides build order for the input dep graph
pub(crate) fn calculate_build_order(graph: &DependencyGraph) -> Result<Vec<String>> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // zeroed all keys
    for pkg in graph.keys() {
        in_degree.entry(pkg.clone()).or_insert(0);
    }

    // now get how many packages depend on A
    for deps in graph.values() {
        for dep in deps {
            *in_degree.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    // recursively visit all graph components
    let mut order = vec![];
    let mut visited = HashSet::new();
    let mut temp_visited = HashSet::new();
    for node in graph.keys() {
        if !visited.contains(node) {
            visit(node, graph, &mut visited, &mut temp_visited, &mut order)?;
        }
    }

    Ok(order)
}

// see https://en.wikipedia.org/wiki/Tree_traversal
fn visit(
    pkg: &str,
    graph: &DependencyGraph,
    visited: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    build_order: &mut Vec<String>,
) -> Result<()> {
    // NOTE(vnepogodin): maybe skip later?
    if visiting.contains(pkg) {
        anyhow::bail!("Circular dep is not allowed: '{pkg}'");
    }
    if visited.contains(pkg) {
        return Ok(());
    }

    // keep track of visited components
    visiting.insert(pkg.into());

    // recursively visit all deps
    if let Some(dependencies) = graph.get(pkg) {
        let mut sorted_deps = dependencies.clone();
        sorted_deps.sort();

        // assuming we have only AUR deps, which were filtered out earlier
        for dep in sorted_deps {
            visit(&dep, graph, visited, visiting, build_order)?;
        }
    }

    // add visited to our build order
    visiting.remove(pkg);
    visited.insert(pkg.into());
    build_order.push(pkg.into());

    Ok(())
}

fn clean_dep_name(dep: &str) -> String {
    dep.split(['>', '<', '=', ':']).next().unwrap_or(dep).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dummy_pkg(name: &str, depends: Vec<&str>, make_depends: Vec<&str>) -> Package {
        Package {
            name: name.to_string(),
            package_base: name.to_string(),
            version: "1.0".to_string(),
            url_path: "".to_string(),
            depends: depends.iter().map(|s| s.to_string()).collect(),
            make_depends: make_depends.iter().map(|s| s.to_string()).collect(),
            opt_depends: vec![],
            check_depends: vec![],
        }
    }

    #[test]
    fn test_clean_dep_name() {
        assert_eq!(clean_dep_name("python>=3.10"), "python");
        assert_eq!(clean_dep_name("libfoo=1.2.3"), "libfoo");
        assert_eq!(clean_dep_name("gcc<9.0"), "gcc");
        assert_eq!(clean_dep_name("abcd"), "abcd");
    }

    #[test]
    fn test_dependency_resolution_and_order() {
        let mut aur_map = HashMap::new();
        aur_map.insert(
            "app".to_string(),
            create_dummy_pkg("app", vec!["lib-a>=1.0", "lib-b"], vec!["cmake"]),
        );
        aur_map.insert("lib-a".to_string(), create_dummy_pkg("lib-a", vec!["lib-common"], vec![]));
        aur_map
            .insert("lib-b".to_string(), create_dummy_pkg("lib-b", vec!["system-glibc"], vec![]));
        aur_map.insert("lib-common".to_string(), create_dummy_pkg("lib-common", vec![], vec![]));

        let targets = vec!["app".to_string()];

        let graph = build_dependency_graph(&targets, &aur_map).unwrap();

        assert!(graph.contains_key("app"));
        assert!(graph.contains_key("lib-a"));
        assert!(graph.contains_key("lib-b"));
        assert!(graph.contains_key("lib-common"));

        let app_deps = graph.get("app").unwrap();
        assert!(app_deps.contains(&"lib-a".to_string()));
        assert!(app_deps.contains(&"lib-b".to_string()));
        assert!(!app_deps.contains(&"cmake".to_string()));

        let libb_deps = graph.get("lib-b").unwrap();
        assert!(libb_deps.is_empty());

        let order = calculate_build_order(&graph).unwrap();

        let idx_common = order.iter().position(|x| x == "lib-common").unwrap();
        let idx_liba = order.iter().position(|x| x == "lib-a").unwrap();
        assert!(idx_common < idx_liba);

        let idx_app = order.iter().position(|x| x == "app").unwrap();
        let idx_libb = order.iter().position(|x| x == "lib-b").unwrap();

        assert!(idx_liba < idx_app);
        assert!(idx_libb < idx_app);
    }

    #[test]
    fn test_circular_dependency() {
        let mut aur_map = HashMap::new();
        // A -> B -> A
        aur_map.insert("pkg-a".to_string(), create_dummy_pkg("pkg-a", vec!["pkg-b"], vec![]));
        aur_map.insert("pkg-b".to_string(), create_dummy_pkg("pkg-b", vec!["pkg-a"], vec![]));

        let targets = vec!["pkg-a".to_string()];
        let graph = build_dependency_graph(&targets, &aur_map).unwrap();

        let result = calculate_build_order(&graph);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("Circular dep"));
    }
}
