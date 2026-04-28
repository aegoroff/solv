use crate::error::Collector;
use crate::{Consume, calculate_percent, ux};
use comfy_table::{Attribute, Cell};
use crossterm::style::Stylize;
use num_format::{Locale, ToFormattedString};
use petgraph::Direction;
use petgraph::algo::DfsSpace;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::prelude::DiGraphMap;
use solp::api::{Solution, SolutionConfiguration};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

trait Validator {
    /// does validation
    fn validate(&mut self, statistic: &mut Statistic);
    /// will return true if validation succeeded false otherwise
    fn validation_result(&self) -> bool;
    /// prints validation results if any
    fn print_results(&self);
}

pub struct Validate {
    show_only_problems: bool,
    errors: RefCell<Collector>,
    statistic: RefCell<Statistic>,
}

#[derive(Default)]
struct Statistic {
    cycles: u64,
    danglings: u64,
    not_found: u64,
    missings: u64,
    parsed: u64,
    not_parsed: u64,
    redundant_refs: u64,
    total: u64,
}

impl Display for Statistic {
    #[allow(clippy::cast_possible_truncation)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", " Statistic:".dark_red().bold())?;

        let mut table = ux::new_table();

        table.set_header([
            Cell::new("Category").add_attribute(Attribute::Bold),
            Cell::new("# Solutions").add_attribute(Attribute::Bold),
            Cell::new("%").add_attribute(Attribute::Bold),
        ]);

        let cycles_percent = calculate_percent(self.cycles as i32, self.total as i32);
        let missings_percent = calculate_percent(self.missings as i32, self.total as i32);
        let danglings_percent = calculate_percent(self.danglings as i32, self.total as i32);
        let not_found_percent = calculate_percent(self.not_found as i32, self.total as i32);
        let redundant_refs_percent =
            calculate_percent(self.redundant_refs as i32, self.total as i32);
        let parsed_percent = calculate_percent(self.parsed as i32, self.total as i32);
        let not_parsed_percent = calculate_percent(self.not_parsed as i32, self.total as i32);
        let total_percent = calculate_percent(self.total as i32, self.total as i32);

        table.add_row([
            Cell::new("Successfully parsed"),
            Cell::new(self.parsed.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{parsed_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row([
            Cell::new("Contain dependencies cycles"),
            Cell::new(self.cycles.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{cycles_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row([
            Cell::new("Contain project configurations outside solution's list"),
            Cell::new(self.missings.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{missings_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row([
            Cell::new("Contain dangling project configurations"),
            Cell::new(self.danglings.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{danglings_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row([
            Cell::new("Contain projects that not exists"),
            Cell::new(self.not_found.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{not_found_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row([
            Cell::new("Contain redundant project references"),
            Cell::new(self.redundant_refs.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{redundant_refs_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row([
            Cell::new("Not parsed"),
            Cell::new(self.not_parsed.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{not_parsed_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        table.add_row(["", "", ""]);
        table.add_row([
            Cell::new("Total"),
            Cell::new(self.total.to_formatted_string(&Locale::en)).add_attribute(Attribute::Italic),
            Cell::new(format!("{total_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        writeln!(f, "{table}")
    }
}

impl Validate {
    #[must_use]
    pub fn new(show_only_problems: bool) -> Self {
        Self {
            show_only_problems,
            errors: RefCell::new(Collector::new()),
            statistic: RefCell::new(Statistic::default()),
        }
    }
}

#[derive(Default)]
struct FixStatistic {
    parsed: u64,
    fixed_solutions: u64,
    fixed_projects: u64,
    removed_refs: u64,
    failed_projects: u64,
    not_parsed: u64,
    total: u64,
}

impl Display for FixStatistic {
    #[allow(clippy::cast_possible_truncation)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", " Fix statistic:".dark_red().bold())?;

        let mut table = ux::new_table();
        table.set_header([
            Cell::new("Category").add_attribute(Attribute::Bold),
            Cell::new("#").add_attribute(Attribute::Bold),
            Cell::new("%").add_attribute(Attribute::Bold),
        ]);

        let parsed_percent = calculate_percent(self.parsed as i32, self.total as i32);
        let fixed_solutions_percent =
            calculate_percent(self.fixed_solutions as i32, self.total as i32);
        let not_parsed_percent = calculate_percent(self.not_parsed as i32, self.total as i32);
        let total_percent = calculate_percent(self.total as i32, self.total as i32);

        table.add_row([
            Cell::new("Successfully parsed solutions"),
            Cell::new(self.parsed.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{parsed_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);
        table.add_row([
            Cell::new("Not parsed solutions"),
            Cell::new(self.not_parsed.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{not_parsed_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);
        table.add_row([
            Cell::new("Solutions with applied fixes"),
            Cell::new(self.fixed_solutions.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(format!("{fixed_solutions_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);
        table.add_row([
            Cell::new("Updated project files"),
            Cell::new(self.fixed_projects.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(String::new()).add_attribute(Attribute::Italic),
        ]);
        table.add_row([
            Cell::new("Failed to update project files"),
            Cell::new(self.failed_projects.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(String::new()).add_attribute(Attribute::Italic),
        ]);
        table.add_row([
            Cell::new("Redundant references removed"),
            Cell::new(self.removed_refs.to_formatted_string(&Locale::en))
                .add_attribute(Attribute::Italic),
            Cell::new(String::new()).add_attribute(Attribute::Italic),
        ]);
        table.add_row(["", "", ""]);
        table.add_row([
            Cell::new("Total solutions"),
            Cell::new(self.total.to_formatted_string(&Locale::en)).add_attribute(Attribute::Italic),
            Cell::new(format!("{total_percent:.2}%")).add_attribute(Attribute::Italic),
        ]);

        writeln!(f, "{table}")
    }
}

pub struct ValidateFix {
    errors: RefCell<Collector>,
    statistic: RefCell<FixStatistic>,
    failed: RefCell<Vec<(PathBuf, String)>>,
}

impl ValidateFix {
    #[must_use]
    pub fn new() -> Self {
        Self {
            errors: RefCell::new(Collector::new()),
            statistic: RefCell::new(FixStatistic::default()),
            failed: RefCell::new(Vec::new()),
        }
    }
}

impl Default for ValidateFix {
    fn default() -> Self {
        Self::new()
    }
}

impl Consume for ValidateFix {
    fn ok(&mut self, solution: &Solution) {
        self.statistic.borrow_mut().parsed += 1;

        let mut detector = Redundants::new(solution);
        let mut unused = Statistic::default();
        detector.validate(&mut unused);

        let mut refs_by_project: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        for redundant in detector.redundants {
            refs_by_project
                .entry(redundant.project)
                .or_default()
                .insert(redundant.redundant_reference.clone());
        }

        let mut solution_was_fixed = false;
        for (project_path, refs) in refs_by_project {
            match remove_redundant_reference_lines(&project_path, &refs) {
                Ok(removed) => {
                    if removed > 0 {
                        let mut stat = self.statistic.borrow_mut();
                        stat.fixed_projects += 1;
                        stat.removed_refs += removed as u64;
                        solution_was_fixed = true;
                    }
                }
                Err(err) => {
                    self.statistic.borrow_mut().failed_projects += 1;
                    self.failed
                        .borrow_mut()
                        .push((project_path, err.to_string()));
                }
            }
        }
        if solution_was_fixed {
            self.statistic.borrow_mut().fixed_solutions += 1;
        }
    }

    fn err(&self, path: &str) {
        self.errors.borrow_mut().add_path(path);
    }
}

impl Display for ValidateFix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut statistic = self.statistic.borrow_mut();
        statistic.not_parsed = self.errors.borrow().count();
        statistic.total = statistic.parsed + statistic.not_parsed;
        write!(f, "{statistic}")?;

        let failed = self.failed.borrow();
        if !failed.is_empty() {
            writeln!(f)?;
            writeln!(
                f,
                " {}",
                "Failed to update project files:".dark_red().bold()
            )?;
            for (path, error) in failed.iter() {
                writeln!(f, "   {}: {}", path.to_string_lossy(), error)?;
            }
        }

        if self.errors.borrow().count() > 0 {
            write!(f, "{}", self.errors.borrow())?;
        }
        Ok(())
    }
}

impl Consume for Validate {
    fn ok(&mut self, solution: &Solution) {
        let mut validators: [Box<dyn Validator>; 5] = [
            Box::new(Cycles::new(solution)),
            Box::new(Danglings::new(solution)),
            Box::new(NotFouund::new(solution)),
            Box::new(Missings::new(solution)),
            Box::new(Redundants::new(solution)),
        ];

        let valid_solution = validators.iter_mut().fold(true, |mut res, validator| {
            validator.validate(&mut self.statistic.borrow_mut());
            res &= validator.validation_result();
            res
        });

        if !self.show_only_problems || !valid_solution {
            ux::print_solution_path(solution.path);
        }
        for v in &validators {
            if !v.validation_result() {
                v.print_results();
            }
        }

        if !self.show_only_problems && valid_solution {
            println!(
                "   {}",
                "No problems found in solution.".dark_green().bold()
            );
            println!();
        }
        if !valid_solution {
            println!();
        }
        self.statistic.borrow_mut().total += 1;
    }

    fn err(&self, path: &str) {
        self.errors.borrow_mut().add_path(path);
    }
}

impl Display for Validate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut statistic = self.statistic.borrow_mut();
        statistic.not_parsed = self.errors.borrow().count();
        statistic.parsed = statistic.total;
        statistic.total += statistic.not_parsed;
        write!(f, "{statistic}")?;
        if self.errors.borrow().count() > 0 {
            write!(f, "{}", self.errors.borrow())
        } else {
            Ok(())
        }
    }
}

struct NotFouund<'a> {
    solution: &'a Solution<'a>,
    bad_paths: BTreeSet<PathBuf>,
}

impl<'a> NotFouund<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self {
            solution,
            bad_paths: BTreeSet::new(),
        }
    }
}

impl Validator for NotFouund<'_> {
    fn validate(&mut self, statistic: &mut Statistic) {
        let dir = crate::parent_of(self.solution.path);
        self.bad_paths = self
            .solution
            .iterate_projects_without_web_sites()
            .filter_map(|p| crate::try_make_local_path(dir, p.path_or_uri))
            .filter_map(|full_path| {
                // we need only not found paths
                full_path.canonicalize().err()?;
                Some(full_path)
            })
            .collect();
        if !self.validation_result() {
            statistic.not_found += 1;
        }
    }

    fn validation_result(&self) -> bool {
        self.bad_paths.is_empty()
    }

    fn print_results(&self) {
        let items = self.bad_paths.iter().filter_map(|p| p.as_path().to_str());
        ux::print_one_column_table(
            "Unexist project path",
            Some(comfy_table::Color::DarkYellow),
            items,
        );
    }
}

struct Danglings<'a> {
    solution: &'a Solution<'a>,
}

impl<'a> Danglings<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self { solution }
    }
}

impl Validator for Danglings<'_> {
    fn validate(&mut self, statistic: &mut Statistic) {
        if !self.validation_result() {
            statistic.danglings += 1;
        }
    }

    fn validation_result(&self) -> bool {
        self.solution.dangling_project_configurations.is_none()
    }

    fn print_results(&self) {
        if let Some(danglings) = &self.solution.dangling_project_configurations {
            ux::print_one_column_table(
                "Dangling project configurations that can be safely removed",
                Some(comfy_table::Color::DarkYellow),
                danglings.iter(),
            );
        }
    }
}

struct Missings<'a> {
    solution: &'a Solution<'a>,
    missings: HashMap<&'a str, Vec<SolutionConfiguration<'a>>>,
}

impl<'a> Missings<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self {
            solution,
            missings: HashMap::new(),
        }
    }
}

impl Validator for Missings<'_> {
    fn validate(&mut self, statistic: &mut Statistic) {
        self.missings = self
            .solution
            .projects
            .iter()
            .filter_map(|p| {
                let mut result = vec![];
                let configurations = p.configurations.as_ref()?;
                for c in configurations {
                    let solution_conf = SolutionConfiguration {
                        configuration: c.solution_configuration,
                        platform: c.platform,
                    };
                    if !self.solution.configurations.contains(&solution_conf) {
                        result.push(solution_conf);
                    }
                }
                if result.is_empty() {
                    None
                } else {
                    Some((p.id, result))
                }
            })
            .collect();

        if !self.validation_result() {
            statistic.missings += 1;
        }
    }

    fn validation_result(&self) -> bool {
        self.missings.is_empty()
    }

    fn print_results(&self) {
        println!("  {}", "Solution contains project configurations that are outside solution's configuration|platform list:".dark_yellow().bold());

        let mut table = ux::new_table();
        table.set_header([
            Cell::new("Project ID").add_attribute(Attribute::Bold),
            Cell::new("Configuration|Platform").add_attribute(Attribute::Bold),
        ]);

        for (id, configs) in &self.missings {
            for config in configs {
                table.add_row([
                    Cell::new(*id),
                    Cell::new(format!("{}|{}", config.configuration, config.platform)),
                ]);
            }
        }

        println!("{table}");
    }
}

struct Cycles<'a> {
    solution: &'a Solution<'a>,
    cycles_detected: bool,
}

impl<'a> Cycles<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self {
            solution,
            cycles_detected: false,
        }
    }
}

impl<'a> Validator for Cycles<'a> {
    fn validate(&mut self, statistic: &mut Statistic) {
        let mut graph = DiGraphMap::<&'a str, ()>::new();
        for to in &self.solution.projects {
            graph.add_node(to.id);
            if let Some(depends_from) = &to.depends_from {
                for from in depends_from {
                    if !graph.contains_node(from) {
                        graph.add_node(from);
                    }
                    graph.add_edge(from, to.id, ());
                }
            }
        }

        let mut space = DfsSpace::new(&graph);
        self.cycles_detected = petgraph::algo::toposort(&graph, Some(&mut space)).is_err();
        if self.cycles_detected {
            statistic.cycles += 1;
        }
    }

    fn validation_result(&self) -> bool {
        !self.cycles_detected
    }

    fn print_results(&self) {
        println!(
            "   {}",
            "Solution contains project dependencies cycles"
                .dark_red()
                .bold()
        );
    }
}

/// A single redundant project reference detected in a project: `project`
/// directly references `redundant_reference`, but the same reference is also
/// reachable transitively through some other direct reference of `project`,
/// so the direct reference can be safely removed.
struct RedundantRef {
    project: PathBuf,
    redundant_reference: String,
}

struct Redundants<'a> {
    solution: &'a Solution<'a>,
    redundants: Vec<RedundantRef>,
}

impl<'a> Redundants<'a> {
    pub fn new(solution: &'a Solution<'a>) -> Self {
        Self {
            solution,
            redundants: Vec::new(),
        }
    }

    /// Builds a directed graph where an edge `from -> to` means
    /// project `to` directly references project `from`
    /// (i.e., `to` depends on `from`).
    fn build_graph(&self) -> DiGraph<PathBuf, String> {
        let projects = crate::collect_msbuild_projects(self.solution);
        let mut graph = DiGraph::<PathBuf, String>::new();
        let mut nodes: HashMap<PathBuf, NodeIndex> = HashMap::new();

        for prj in projects {
            let to_path = prj.path.canonicalize().unwrap_or_else(|_| prj.path.clone());
            let to = Self::ensure_node(&mut graph, &mut nodes, &to_path);

            let Some(project) = prj.project else { continue };
            let Some(item_groups) = project.item_group else {
                continue;
            };
            let Some(parent) = prj.path.parent() else {
                continue;
            };

            for ig in item_groups {
                let Some(refs) = ig.project_reference else {
                    continue;
                };
                for reference in refs {
                    let include = reference.include.clone();
                    #[cfg(target_os = "windows")]
                    let normalized_include = include.as_str();
                    #[cfg(not(target_os = "windows"))]
                    let normalized_include = decorate_path(&include);

                    let joined = parent.join(normalized_include);
                    let Ok(reference_path) = joined.canonicalize() else {
                        continue;
                    };

                    let from = Self::ensure_node(&mut graph, &mut nodes, &reference_path);
                    // do not create self-loops
                    if from == to {
                        continue;
                    }
                    if graph.find_edge(from, to).is_none() {
                        graph.add_edge(from, to, include);
                    }
                }
            }
        }
        graph
    }

    fn ensure_node(
        graph: &mut DiGraph<PathBuf, String>,
        nodes: &mut HashMap<PathBuf, NodeIndex>,
        path: &Path,
    ) -> NodeIndex {
        if let Some(ix) = nodes.get(path) {
            *ix
        } else {
            let ix = graph.add_node(path.to_path_buf());
            nodes.insert(path.to_path_buf(), ix);
            ix
        }
    }

    /// Returns true if `target` is reachable from `start` without visiting
    /// `forbidden`. This is used to verify whether a direct reference
    /// `start -> forbidden` is still implied transitively through another
    /// predecessor of `forbidden` after effectively removing that edge.
    fn has_path_avoiding_node(
        graph: &DiGraph<PathBuf, String>,
        start: NodeIndex,
        target: NodeIndex,
        forbidden: NodeIndex,
    ) -> bool {
        if start == forbidden || target == forbidden {
            return false;
        }
        if start == target {
            return true;
        }

        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut stack: Vec<NodeIndex> = vec![start];

        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            for next in graph.neighbors_directed(current, Direction::Outgoing) {
                if next == forbidden {
                    continue;
                }
                if next == target {
                    return true;
                }
                if !visited.contains(&next) {
                    stack.push(next);
                }
            }
        }

        false
    }

    /// For each node N, looks at all its direct predecessors P (i.e., projects
    /// directly referenced by N). An edge `p -> N` is considered redundant if
    /// there exists another direct predecessor `p'` of N (with `p' != p`) such
    /// that there is a path `p -> ... -> p'` in the graph. In that case, N
    /// already receives a transitive dependency on `p` through `p'`, so the
    /// direct reference `p -> N` is unnecessary.
    fn find_redundants(graph: &DiGraph<PathBuf, String>) -> Vec<RedundantRef> {
        let mut result: Vec<RedundantRef> = Vec::new();

        for node in graph.node_indices() {
            let direct_preds: Vec<NodeIndex> = graph
                .neighbors_directed(node, Direction::Incoming)
                .collect();
            if direct_preds.len() < 2 {
                continue;
            }

            for &candidate in &direct_preds {
                // `candidate -> node` is redundant when another direct
                // predecessor `other` of `node` already depends (directly or
                // transitively) on `candidate`, i.e., there is a path
                // `candidate -> ... -> other`. In that case `node` will reach
                // `candidate` transitively through `other` and the direct
                // `candidate -> node` edge is unnecessary.
                let reachable_via_other = direct_preds
                    .iter()
                    .filter(|&&other| other != candidate)
                    .any(|&other| Self::has_path_avoiding_node(graph, candidate, other, node));

                if reachable_via_other {
                    let Some(edge) = graph.find_edge(candidate, node) else {
                        continue;
                    };
                    result.push(RedundantRef {
                        project: graph[node].clone(),
                        redundant_reference: graph[edge].clone(),
                    });
                }
            }
        }

        result.sort_by(|a, b| {
            a.project
                .cmp(&b.project)
                .then_with(|| a.redundant_reference.cmp(&b.redundant_reference))
        });
        result
    }
}

impl Validator for Redundants<'_> {
    fn validate(&mut self, statistic: &mut Statistic) {
        let graph = self.build_graph();
        self.redundants = Self::find_redundants(&graph);
        if !self.validation_result() {
            statistic.redundant_refs += 1;
        }
    }

    fn validation_result(&self) -> bool {
        self.redundants.is_empty()
    }

    fn print_results(&self) {
        if self.redundants.is_empty() {
            return;
        }
        println!(
            "  {}",
            "Solution contains redundant project references that can be replaced by transitive dependencies:"
                .dark_yellow()
                .bold()
        );

        // `self.redundants` is sorted by (project, redundant_reference), so a
        // linear pass is enough to build stable project groups.
        let mut current_project: Option<&Path> = None;
        let mut current_rows: Vec<String> = Vec::new();
        for r in &self.redundants {
            if current_project != Some(r.project.as_path()) {
                if let Some(project) = current_project {
                    ux::print_one_column_table(
                        &project.to_string_lossy(),
                        Some(comfy_table::Color::DarkBlue),
                        current_rows.drain(..),
                    );
                }
                current_project = Some(r.project.as_path());
            }
            current_rows.push(r.redundant_reference.clone());
        }

        if let Some(project) = current_project {
            ux::print_one_column_table(
                &project.to_string_lossy(),
                Some(comfy_table::Color::DarkBlue),
                current_rows.into_iter(),
            );
        }
        println!();
    }
}

fn remove_redundant_reference_lines(
    path: &Path,
    redundant_refs: &HashSet<String>,
) -> std::io::Result<usize> {
    if redundant_refs.is_empty() {
        return Ok(0);
    }

    let input = fs::read(path)?;
    let spans = find_redundant_reference_spans(&input, redundant_refs);
    if spans.is_empty() {
        return Ok(0);
    }

    // Build output by removing each span. Also consume the leading whitespace
    // before the tag start and the trailing newline after the tag end, when
    // doing so would only drop whitespace-only content (so that whole lines
    // occupied solely by a removed `<ProjectReference ... />` element vanish
    // cleanly, including in the multi-line attribute case).
    let mut effective_spans: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
    let mut output_size = input.len();
    let mut prev_end = 0usize;
    for (start, end) in &spans {
        let extended_start = expand_start_over_line_whitespace(&input, *start, prev_end);
        let extended_end = expand_end_over_line_whitespace(&input, *end);
        output_size -= extended_end - extended_start;
        effective_spans.push((extended_start, extended_end));
        prev_end = extended_end;
    }

    let mut output = Vec::with_capacity(output_size);
    let mut cursor = 0usize;
    for (start, end) in &effective_spans {
        output.extend_from_slice(&input[cursor..*start]);
        cursor = *end;
    }
    output.extend_from_slice(&input[cursor..]);

    fs::write(path, &output)?;
    Ok(spans.len())
}

/// Walks back from `start` while the preceding bytes on the same line are
/// horizontal whitespace, so that a leading indentation in front of the
/// `<ProjectReference ...>` element gets removed together with the element
/// itself. The walk-back is bounded by `lower_bound` (typically the previous
/// span's end) to avoid clobbering previously kept content.
fn expand_start_over_line_whitespace(input: &[u8], mut start: usize, lower_bound: usize) -> usize {
    while start > lower_bound {
        let c = input[start - 1];
        if c == b'\n' {
            break;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            start -= 1;
            continue;
        }
        // Non-whitespace content sits on the same line before the tag; we
        // must keep that content, so do not extend the removed range.
        return start;
    }
    start
}

/// Walks forward from `end` while the trailing bytes on the same line are
/// horizontal whitespace, and consumes a single line terminator if found, so
/// that an empty line left behind by the removed element disappears too.
fn expand_end_over_line_whitespace(input: &[u8], mut end: usize) -> usize {
    let len = input.len();
    let saved = end;
    while end < len {
        let c = input[end];
        if c == b' ' || c == b'\t' || c == b'\r' {
            end += 1;
            continue;
        }
        if c == b'\n' {
            return end + 1;
        }
        // Non-whitespace content sits on the same line after the tag; keep it.
        return saved;
    }
    end
}

/// Scans the raw bytes of an MSBuild project file for `<ProjectReference ...>`
/// elements (both self-closing and with an explicit `</ProjectReference>`
/// closing tag, possibly spanning multiple lines as XML allows) whose
/// `Include` attribute value is contained in `redundant_refs`. Returns the
/// byte ranges `[start, end)` of every such element occurrence, in input
/// order, where `start` is the position of `<` and `end` is one past the
/// final `>` of the element.
fn find_redundant_reference_spans(
    input: &[u8],
    redundant_refs: &HashSet<String>,
) -> Vec<(usize, usize)> {
    const TAG: &[u8] = b"<ProjectReference";
    const CLOSE_TAG: &[u8] = b"</ProjectReference>";
    let len = input.len();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;

    while i + TAG.len() <= len {
        if &input[i..i + TAG.len()] != TAG {
            i += 1;
            continue;
        }
        // The tag name must be terminated by whitespace, '/', or '>' to avoid
        // matching things like `<ProjectReferenceXxx`.
        let after = input.get(i + TAG.len()).copied().unwrap_or(0);
        if !(after.is_ascii_whitespace() || after == b'/' || after == b'>') {
            i += 1;
            continue;
        }

        // Find end of opening tag, accounting for quoted attribute values that
        // may contain '>' characters.
        let mut j = i + TAG.len();
        let mut in_quote: Option<u8> = None;
        let mut self_closing = false;
        let mut found_close = false;
        while j < len {
            let c = input[j];
            if let Some(q) = in_quote {
                if c == q {
                    in_quote = None;
                }
                j += 1;
            } else if c == b'"' || c == b'\'' {
                in_quote = Some(c);
                j += 1;
            } else if c == b'>' {
                if j > 0 && input[j - 1] == b'/' {
                    self_closing = true;
                }
                j += 1;
                found_close = true;
                break;
            } else {
                j += 1;
            }
        }
        if !found_close {
            // Malformed input — give up.
            break;
        }
        let opening_tag_end = j; // one past '>'

        // Slice of the attribute list (between `<ProjectReference` and the
        // terminating `>` or `/>`).
        let attrs_start = i + TAG.len();
        let attrs_end = if self_closing {
            opening_tag_end - 2
        } else {
            opening_tag_end - 1
        };
        let include = extract_include_value(&input[attrs_start..attrs_end]);

        let span_end = if self_closing {
            opening_tag_end
        } else {
            // Find matching `</ProjectReference>`. MSBuild project files do
            // not nest `<ProjectReference>` inside another `<ProjectReference>`,
            // so a plain forward scan is sufficient.
            let mut k = opening_tag_end;
            let mut close_pos = None;
            while k + CLOSE_TAG.len() <= len {
                if &input[k..k + CLOSE_TAG.len()] == CLOSE_TAG {
                    close_pos = Some(k + CLOSE_TAG.len());
                    break;
                }
                k += 1;
            }
            if let Some(p) = close_pos {
                p
            } else {
                // Malformed: skip past this opening tag and continue.
                i = opening_tag_end;
                continue;
            }
        };

        if let Some(inc) = include
            && redundant_refs.contains(inc)
        {
            spans.push((i, span_end));
        }

        i = span_end;
    }
    spans
}

fn extract_include_value(line: &[u8]) -> Option<&str> {
    let len = line.len();
    let include_bytes: &[u8] = b"Include";
    let mut i = 0;

    // Find "Include" without creating intermediate slices
    while i + include_bytes.len() <= len {
        // Fast path: check first byte 'I' before full comparison
        if line[i] == b'I' && &line[i..i + include_bytes.len()] == include_bytes {
            i += include_bytes.len();

            // Skip whitespace after "Include"
            while i < len && line[i].is_ascii_whitespace() {
                i += 1;
            }
            // Must be followed by '='
            if i < len && line[i] == b'=' {
                i += 1;

                // Skip whitespace after "="
                while i < len && line[i].is_ascii_whitespace() {
                    i += 1;
                }
                // Must be followed by a quote
                if i < len {
                    let quote = line[i];
                    if quote == b'"' || quote == b'\'' {
                        i += 1;
                        let value_start = i;
                        // Find closing quote
                        while i < len && line[i] != quote {
                            i += 1;
                        }
                        if i < len {
                            return std::str::from_utf8(&line[value_start..i]).ok();
                        }
                    }
                }
            }
            // Not a valid Include="..." pattern, continue searching
        }
        i += 1;
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn decorate_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn integration_test_correct_solution() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok(&solution);

        // Assert
    }

    #[test]
    fn integration_test_solution_with_danglings() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_DANGLINGS).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok(&solution);

        // Assert
    }

    #[test]
    fn integration_test_solution_with_missings() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_MISSING_PROJECT_CONFIGS).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok(&solution);

        // Assert
    }

    #[test]
    fn integration_test_solution_with_cycles() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_CYCLES).unwrap();
        let mut validator = Validate::new(false);

        // Act
        validator.ok(&solution);

        // Assert
    }

    #[test]
    fn dangling_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Danglings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(validator.validation_result());
    }

    #[test]
    fn cycles_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Cycles::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(validator.validation_result());
        assert_eq!(0, statistic.cycles);
    }

    #[test]
    fn cycles_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_CYCLES).unwrap();
        let mut validator = Cycles::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(!validator.validation_result());
        assert_eq!(1, statistic.cycles);
    }

    #[test]
    fn missing_validation_correct() {
        // Arrange
        let solution = solp::parse_str(CORRECT_SOLUTION).unwrap();
        let mut validator = Missings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(validator.validation_result());
        assert_eq!(0, statistic.missings);
    }

    #[test]
    fn missing_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_MISSING_PROJECT_CONFIGS).unwrap();
        let mut validator = Missings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(!validator.validation_result());
        assert_eq!(1, statistic.missings);
    }

    #[test]
    fn dangling_validation_incorrect() {
        // Arrange
        let solution = solp::parse_str(SOLUTION_WITH_DANGLINGS).unwrap();
        let mut validator = Danglings::new(&solution);
        let mut statistic = Statistic::default();

        // Act
        validator.validate(&mut statistic);

        // Assert
        assert!(!validator.validation_result());
        assert_eq!(1, statistic.danglings);
    }

    #[test]
    fn print_statistic_test() {
        // Arrange
        let s = Statistic::default();

        // Act
        println!("{s}");

        // Assert
    }

    fn add_node(graph: &mut DiGraph<PathBuf, String>, name: &str) -> NodeIndex {
        graph.add_node(PathBuf::from(name))
    }

    #[test]
    fn redundants_empty_graph_has_no_redundants() {
        // Arrange
        let graph = DiGraph::<PathBuf, String>::new();

        // Act
        let redundants = Redundants::find_redundants(&graph);

        // Assert
        assert!(redundants.is_empty());
    }

    #[test]
    fn redundants_single_dependency_has_no_redundants() {
        // Arrange
        let mut graph = DiGraph::<PathBuf, String>::new();
        let a = add_node(&mut graph, "a");
        let b = add_node(&mut graph, "b");
        graph.add_edge(a, b, "a->b".to_owned());

        // Act
        let redundants = Redundants::find_redundants(&graph);

        // Assert
        assert!(redundants.is_empty());
    }

    #[test]
    fn redundants_simple_triangle_detects_redundant() {
        // Arrange:
        //   a -> b, a -> c, b -> c
        // 'a' is a direct ref of 'c', but already reachable transitively
        // through 'b' (a -> b -> c). So the direct edge a -> c is redundant.
        let mut graph = DiGraph::<PathBuf, String>::new();
        let a = add_node(&mut graph, "a");
        let b = add_node(&mut graph, "b");
        let c = add_node(&mut graph, "c");
        graph.add_edge(a, b, "a->b".to_owned());
        graph.add_edge(a, c, "..\\A\\A.csproj".to_owned());
        graph.add_edge(b, c, "..\\B\\B.csproj".to_owned());

        // Act
        let redundants = Redundants::find_redundants(&graph);

        // Assert
        assert_eq!(1, redundants.len());
        assert_eq!(PathBuf::from("c"), redundants[0].project);
        assert_eq!("..\\A\\A.csproj", redundants[0].redundant_reference);
    }

    #[test]
    fn redundants_independent_refs_are_not_redundant() {
        // Arrange:
        //   a -> c, b -> c (a and b are independent)
        let mut graph = DiGraph::<PathBuf, String>::new();
        let a = add_node(&mut graph, "a");
        let b = add_node(&mut graph, "b");
        let c = add_node(&mut graph, "c");
        graph.add_edge(a, c, "a->c".to_owned());
        graph.add_edge(b, c, "b->c".to_owned());

        // Act
        let redundants = Redundants::find_redundants(&graph);

        // Assert
        assert!(redundants.is_empty());
    }

    #[test]
    fn redundants_deep_chain() {
        // Arrange:
        //   a -> b -> c -> d, and a -> d (direct)
        // 'a' is a direct ref of 'd', reachable transitively via 'c' (a -> b -> c -> d).
        let mut graph = DiGraph::<PathBuf, String>::new();
        let a = add_node(&mut graph, "a");
        let b = add_node(&mut graph, "b");
        let c = add_node(&mut graph, "c");
        let d = add_node(&mut graph, "d");
        graph.add_edge(a, b, "a->b".to_owned());
        graph.add_edge(b, c, "b->c".to_owned());
        graph.add_edge(c, d, "c->d".to_owned());
        graph.add_edge(a, d, "..\\A\\A.csproj".to_owned());

        // Act
        let redundants = Redundants::find_redundants(&graph);

        // Assert
        assert_eq!(1, redundants.len());
        assert_eq!(PathBuf::from("d"), redundants[0].project);
        assert_eq!("..\\A\\A.csproj", redundants[0].redundant_reference);
    }

    #[test]
    fn redundants_path_via_target_node_is_not_redundant() {
        // Arrange:
        //   a -> n, b -> n, n -> b
        // There is a path a -> b, but only through n. Removing a -> n breaks
        // that path, so a -> n must not be considered redundant.
        let mut graph = DiGraph::<PathBuf, String>::new();
        let a = add_node(&mut graph, "a");
        let b = add_node(&mut graph, "b");
        let n = add_node(&mut graph, "n");
        graph.add_edge(a, n, "a->n".to_owned());
        graph.add_edge(b, n, "b->n".to_owned());
        graph.add_edge(n, b, "n->b".to_owned());

        // Act
        let redundants = Redundants::find_redundants(&graph);

        // Assert
        assert!(redundants.is_empty());
    }

    #[test]
    fn redundants_normalize_real_paths_and_detect_redundancy() {
        // Arrange
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-redundants-{uniq}"));
        let a_dir = root.join("A");
        let b_dir = root.join("B");
        let app_dir = root.join("App");
        let shared_dir = root.join("Shared");

        fs::create_dir_all(&a_dir).unwrap();
        fs::create_dir_all(&b_dir).unwrap();
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&shared_dir).unwrap();

        fs::write(
            shared_dir.join("Shared.csproj"),
            r#"<Project Sdk="Microsoft.NET.Sdk"></Project>"#,
        )
        .unwrap();

        fs::write(
            a_dir.join("A.csproj"),
            r#"
<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <ProjectReference Include="..\Shared\Shared.csproj" />
  </ItemGroup>
</Project>
"#,
        )
        .unwrap();

        fs::write(
            b_dir.join("B.csproj"),
            r#"
<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <ProjectReference Include="..\Shared\Shared.csproj" />
  </ItemGroup>
</Project>
"#,
        )
        .unwrap();

        fs::write(
            app_dir.join("App.csproj"),
            r#"
<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <ProjectReference Include="..\A\A.csproj" />
    <ProjectReference Include="..\Shared\Shared.csproj" />
  </ItemGroup>
</Project>
"#,
        )
        .unwrap();

        let sln = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio Version 17
Project("{{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}}") = "App", "App/App.csproj", "{{A1111111-1111-1111-1111-111111111111}}"
EndProject
Project("{{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}}") = "A", "A/A.csproj", "{{A2222222-2222-2222-2222-222222222222}}"
EndProject
Project("{{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}}") = "B", "B/B.csproj", "{{A3333333-3333-3333-3333-333333333333}}"
EndProject
Project("{{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}}") = "Shared", "Shared/../Shared/Shared.csproj", "{{A4444444-4444-4444-4444-444444444444}}"
EndProject
Global
    GlobalSection(SolutionConfigurationPlatforms) = preSolution
        Debug|Any CPU = Debug|Any CPU
    EndGlobalSection
    GlobalSection(ProjectConfigurationPlatforms) = postSolution
        {{A1111111-1111-1111-1111-111111111111}}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {{A2222222-2222-2222-2222-222222222222}}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {{A3333333-3333-3333-3333-333333333333}}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {{A4444444-4444-4444-4444-444444444444}}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
    EndGlobalSection
EndGlobal
"#;

        let mut solution = solp::parse_str(sln).unwrap();
        let sln_path = root.join("test.sln");
        let leaked_path: &'static str =
            Box::leak(sln_path.to_string_lossy().into_owned().into_boxed_str());
        solution.path = leaked_path;

        let mut validator = Redundants::new(&solution);

        // Act
        let graph = validator.build_graph();
        validator.redundants = Redundants::find_redundants(&graph);

        // Assert
        assert_eq!(4, graph.node_count());

        let app_path = app_dir.join("App.csproj").canonicalize().unwrap();
        assert!(
            validator
                .redundants
                .iter()
                .any(|r| r.project == app_path
                    && r.redundant_reference == "..\\Shared\\Shared.csproj")
        );

        // Cleanup
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn remove_redundant_reference_lines_removes_only_target_line() {
        // Arrange
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-fix-lines-{uniq}"));
        fs::create_dir_all(&root).unwrap();
        let project_path = root.join("App.csproj");
        let original = concat!(
            "<Project>\r\n",
            "  <ItemGroup>\r\n",
            "    <ProjectReference Include=\"..\\A\\A.csproj\" />\r\n",
            "    <ProjectReference Include=\"..\\B\\B.csproj\" />\r\n",
            "  </ItemGroup>\r\n",
            "</Project>\r\n",
        );
        fs::write(&project_path, original).unwrap();
        let mut refs = HashSet::new();
        refs.insert("..\\A\\A.csproj".to_string());

        // Act
        let removed = remove_redundant_reference_lines(&project_path, &refs).unwrap();
        let updated = fs::read(&project_path).unwrap();

        // Assert
        assert_eq!(1, removed);
        assert_eq!(
            updated,
            concat!(
                "<Project>\r\n",
                "  <ItemGroup>\r\n",
                "    <ProjectReference Include=\"..\\B\\B.csproj\" />\r\n",
                "  </ItemGroup>\r\n",
                "</Project>\r\n",
            )
            .as_bytes()
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn remove_redundant_reference_lines_removes_multiline_self_closing_tag() {
        // Arrange: <ProjectReference> spans three physical lines.
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-fix-multiline-{uniq}"));
        fs::create_dir_all(&root).unwrap();
        let project_path = root.join("App.csproj");
        let original = concat!(
            "<Project>\n",
            "  <ItemGroup>\n",
            "    <ProjectReference\n",
            "        Include=\"..\\A\\A.csproj\"\n",
            "    />\n",
            "    <ProjectReference Include=\"..\\B\\B.csproj\" />\n",
            "  </ItemGroup>\n",
            "</Project>\n",
        );
        fs::write(&project_path, original).unwrap();
        let mut refs = HashSet::new();
        refs.insert("..\\A\\A.csproj".to_string());

        // Act
        let removed = remove_redundant_reference_lines(&project_path, &refs).unwrap();
        let updated = fs::read(&project_path).unwrap();

        // Assert
        assert_eq!(1, removed);
        assert_eq!(
            updated,
            concat!(
                "<Project>\n",
                "  <ItemGroup>\n",
                "    <ProjectReference Include=\"..\\B\\B.csproj\" />\n",
                "  </ItemGroup>\n",
                "</Project>\n",
            )
            .as_bytes()
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn remove_redundant_reference_lines_removes_multiline_with_explicit_close_tag() {
        // Arrange: <ProjectReference ...> ... </ProjectReference> spans 4 lines.
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-fix-multiline-close-{uniq}"));
        fs::create_dir_all(&root).unwrap();
        let project_path = root.join("App.csproj");
        let original = concat!(
            "<Project>\r\n",
            "  <ItemGroup>\r\n",
            "    <ProjectReference\r\n",
            "        Include=\"..\\A\\A.csproj\">\r\n",
            "      <Private>true</Private>\r\n",
            "    </ProjectReference>\r\n",
            "    <ProjectReference Include=\"..\\B\\B.csproj\" />\r\n",
            "  </ItemGroup>\r\n",
            "</Project>\r\n",
        );
        fs::write(&project_path, original).unwrap();
        let mut refs = HashSet::new();
        refs.insert("..\\A\\A.csproj".to_string());

        // Act
        let removed = remove_redundant_reference_lines(&project_path, &refs).unwrap();
        let updated = fs::read(&project_path).unwrap();

        // Assert
        assert_eq!(1, removed);
        assert_eq!(
            updated,
            concat!(
                "<Project>\r\n",
                "  <ItemGroup>\r\n",
                "    <ProjectReference Include=\"..\\B\\B.csproj\" />\r\n",
                "  </ItemGroup>\r\n",
                "</Project>\r\n",
            )
            .as_bytes()
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn remove_redundant_reference_lines_keeps_file_unchanged_without_match() {
        // Arrange
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-fix-lines-noop-{uniq}"));
        fs::create_dir_all(&root).unwrap();
        let project_path = root.join("App.csproj");
        let original = concat!(
            "<Project>\n",
            "  <ItemGroup>\n",
            "    <ProjectReference Include=\"..\\B\\B.csproj\" />\n",
            "  </ItemGroup>\n",
            "</Project>\n",
        );
        fs::write(&project_path, original).unwrap();
        let before = fs::read(&project_path).unwrap();
        let mut refs = HashSet::new();
        refs.insert("..\\A\\A.csproj".to_string());

        // Act
        let removed = remove_redundant_reference_lines(&project_path, &refs).unwrap();
        let after = fs::read(&project_path).unwrap();

        // Assert
        assert_eq!(0, removed);
        assert_eq!(before, after);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn integration_test_validate_fix_removes_redundant_reference() {
        // Arrange: create a realistic project structure with a redundant reference
        // Graph: Shared -> A -> App
        // App directly references both A and Shared, but Shared is reachable transitively
        // through A, so App's direct reference to Shared is redundant.
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("solv-validatefix-{uniq}"));
        let shared_dir = root.join("Shared");
        let a_dir = root.join("A");
        let app_dir = root.join("App");

        fs::create_dir_all(&shared_dir).unwrap();
        fs::create_dir_all(&a_dir).unwrap();
        fs::create_dir_all(&app_dir).unwrap();

        // Shared project - no dependencies
        fs::write(
            shared_dir.join("Shared.csproj"),
            r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net6.0</TargetFramework></PropertyGroup></Project>"#,
        )
        .unwrap();

        // A project - references Shared
        fs::write(
            a_dir.join("A.csproj"),
            concat!(
                "<Project Sdk=\"Microsoft.NET.Sdk\">\n",
                "  <PropertyGroup><TargetFramework>net6.0</TargetFramework></PropertyGroup>\n",
                "  <ItemGroup>\n",
                "    <ProjectReference Include=\"..\\Shared\\Shared.csproj\" />\n",
                "  </ItemGroup>\n",
                "</Project>\n",
            ),
        )
        .unwrap();

        // App project - references both A and Shared (Shared is redundant)
        let app_original = concat!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n",
            "  <PropertyGroup><TargetFramework>net6.0</TargetFramework></PropertyGroup>\n",
            "  <ItemGroup>\n",
            "    <ProjectReference Include=\"..\\A\\A.csproj\" />\n",
            "    <ProjectReference Include=\"..\\Shared\\Shared.csproj\" />\n",
            "  </ItemGroup>\n",
            "</Project>\n",
        );
        fs::write(app_dir.join("App.csproj"), app_original).unwrap();

        // Solution file
        let sln = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio Version 17
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "App", "App/App.csproj", "{A1111111-1111-1111-1111-111111111111}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "A", "A/A.csproj", "{A2222222-2222-2222-2222-222222222222}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "Shared", "Shared/Shared.csproj", "{A4444444-4444-4444-4444-444444444444}"
EndProject
Global
    GlobalSection(SolutionConfigurationPlatforms) = preSolution
        Debug|Any CPU = Debug|Any CPU
    EndGlobalSection
    GlobalSection(ProjectConfigurationPlatforms) = postSolution
        {A1111111-1111-1111-1111-111111111111}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {A2222222-2222-2222-2222-222222222222}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {A4444444-4444-4444-4444-444444444444}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
    EndGlobalSection
EndGlobal
"#;

        let mut solution = solp::parse_str(sln).unwrap();
        let sln_path = root.join("test.sln");
        let leaked_path: &'static str =
            Box::leak(sln_path.to_string_lossy().into_owned().into_boxed_str());
        solution.path = leaked_path;

        // Act
        let mut validator = ValidateFix::new();
        validator.ok(&solution);

        // Assert
        // the redundant reference was removed from App.csproj
        let app_updated = fs::read_to_string(app_dir.join("App.csproj")).unwrap();
        assert!(
            !app_updated.contains("Shared.csproj"),
            "App.csproj should not contain reference to Shared.csproj after fix"
        );
        assert!(
            app_updated.contains("A.csproj"),
            "App.csproj should still contain reference to A.csproj"
        );

        // statistics
        assert_eq!(
            validator.statistic.borrow().fixed_projects,
            1,
            "Should be one fixed project"
        );
        assert_eq!(
            validator.statistic.borrow().removed_refs,
            1,
            "Should be one removed ref"
        );

        // Cleanup
        fs::remove_dir_all(&root).unwrap();
    }

    const CORRECT_SOLUTION: &str = r#"
Microsoft Visual Studio Solution File, Format Version 8.00
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest", "gtest.vcproj", "{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_main", "gtest_main.vcproj", "{3AF54C8A-10BF-4332-9147-F68ED9862032}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_unittest", "gtest_unittest.vcproj", "{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_prod_test", "gtest_prod_test.vcproj", "{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Global
	GlobalSection(SolutionConfiguration) = preSolution
		Debug = Debug
		Release = Release
	EndGlobalSection
	GlobalSection(ProjectConfiguration) = postSolution
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.ActiveCfg = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.Build.0 = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.ActiveCfg = Release|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.Build.0 = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.ActiveCfg = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.Build.0 = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.ActiveCfg = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.Build.0 = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.ActiveCfg = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.Build.0 = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.ActiveCfg = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.Build.0 = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.ActiveCfg = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.Build.0 = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.ActiveCfg = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.Build.0 = Release|Win32
	EndGlobalSection
	GlobalSection(ExtensibilityGlobals) = postSolution
	EndGlobalSection
	GlobalSection(ExtensibilityAddIns) = postSolution
	EndGlobalSection
EndGlobal
"#;

    const SOLUTION_WITH_MISSING_PROJECT_CONFIGS: &str = r#"
Microsoft Visual Studio Solution File, Format Version 11.00
# Visual Studio 2010
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "a", "a\a.csproj", "{78965571-A6C2-4161-95B1-813B46610EA7}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "b", "b\b.csproj", "{D9523F4D-6CB7-4431-85F6-8122F55EB144}"
EndProject
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|Any CPU = Debug|Any CPU
		Release|Any CPU = Release|Any CPU
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|x86.ActiveCfg = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Debug|x86.Build.0 = Debug|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{78965571-A6C2-4161-95B1-813B46610EA7}.Release|Any CPU.Build.0 = Release|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|x86.ActiveCfg = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Debug|x86.Build.0 = Debug|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{D9523F4D-6CB7-4431-85F6-8122F55EB144}.Release|Any CPU.Build.0 = Release|Any CPU
	EndGlobalSection
	GlobalSection(SolutionProperties) = preSolution
		HideSolutionNode = FALSE
	EndGlobalSection
EndGlobal
"#;

    const SOLUTION_WITH_DANGLINGS: &str = r#"
Microsoft Visual Studio Solution File, Format Version 8.00
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest", "gtest.vcproj", "{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_main", "gtest_main.vcproj", "{3AF54C8A-10BF-4332-9147-F68ED9862032}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "gtest_unittest", "gtest_unittest.vcproj", "{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}"
	ProjectSection(ProjectDependencies) = postProject
	EndProjectSection
EndProject
Global
	GlobalSection(SolutionConfiguration) = preSolution
		Debug = Debug
		Release = Release
	EndGlobalSection
	GlobalSection(ProjectConfiguration) = postSolution
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.ActiveCfg = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Debug.Build.0 = Debug|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.ActiveCfg = Release|Win32
		{C8F6C172-56F2-4E76-B5FA-C3B423B31BE7}.Release.Build.0 = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.ActiveCfg = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Debug.Build.0 = Debug|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.ActiveCfg = Release|Win32
		{3AF54C8A-10BF-4332-9147-F68ED9862032}.Release.Build.0 = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.ActiveCfg = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Debug.Build.0 = Debug|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.ActiveCfg = Release|Win32
		{4D9FDFB5-986A-4139-823C-F4EE0ED481A1}.Release.Build.0 = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.ActiveCfg = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Debug.Build.0 = Debug|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.ActiveCfg = Release|Win32
		{24848551-EF4F-47E8-9A9D-EA4D49BC3ECA}.Release.Build.0 = Release|Win32
	EndGlobalSection
	GlobalSection(ExtensibilityGlobals) = postSolution
	EndGlobalSection
	GlobalSection(ExtensibilityAddIns) = postSolution
	EndGlobalSection
EndGlobal
"#;

    const SOLUTION_WITH_CYCLES: &str = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio 15
VisualStudioVersion = 15.0.26403.0
MinimumVisualStudioVersion = 10.0.40219.1
Project("{930C7802-8A8C-48F9-8165-68863BCCD9DD}") = "logviewer.install", "logviewer.install\logviewer.install.wixproj", "{27060CA7-FB29-42BC-BA66-7FC80D498354}"
	ProjectSection(ProjectDependencies) = postProject
		{405827CB-84E1-46F3-82C9-D889892645AC} = {405827CB-84E1-46F3-82C9-D889892645AC}
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D} = {CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}
	EndProjectSection
EndProject
Project("{930C7802-8A8C-48F9-8165-68863BCCD9DD}") = "logviewer.install.bootstrap", "logviewer.install.bootstrap\logviewer.install.bootstrap.wixproj", "{1C0ED62B-D506-4E72-BBC2-A50D3926466E}"
	ProjectSection(ProjectDependencies) = postProject
		{27060CA7-FB29-42BC-BA66-7FC80D498354} = {27060CA7-FB29-42BC-BA66-7FC80D498354}
	EndProjectSection
EndProject
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = "solution items", "solution items", "{3B960F8F-AD5D-45E7-92C0-05B65E200AC4}"
	ProjectSection(SolutionItems) = preProject
		.editorconfig = .editorconfig
		appveyor.yml = appveyor.yml
		logviewer.xml = logviewer.xml
		WiX.msbuild = WiX.msbuild
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.tests", "logviewer.tests\logviewer.tests.csproj", "{939DD379-CDC8-47EF-8D37-0E5E71D99D30}"
	ProjectSection(ProjectDependencies) = postProject
		{383C08FC-9CAC-42E5-9B02-471561479A74} = {383C08FC-9CAC-42E5-9B02-471561479A74}
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.logic", "logviewer.logic\logviewer.logic.csproj", "{383C08FC-9CAC-42E5-9B02-471561479A74}"
	ProjectSection(ProjectDependencies) = postProject
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30} = {939DD379-CDC8-47EF-8D37-0E5E71D99D30}
	EndProjectSection
EndProject
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = ".nuget", ".nuget", "{B720ED85-58CF-4840-B1AE-55B0049212CC}"
	ProjectSection(SolutionItems) = preProject
		.nuget\NuGet.Config = .nuget\NuGet.Config
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.engine", "logviewer.engine\logviewer.engine.csproj", "{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.install.mca", "logviewer.install.mca\logviewer.install.mca.csproj", "{405827CB-84E1-46F3-82C9-D889892645AC}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.ui", "logviewer.ui\logviewer.ui.csproj", "{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.bench", "logviewer.bench\logviewer.bench.csproj", "{75E0C034-44C8-461B-A677-9A19566FE393}"
EndProject
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|Any CPU = Debug|Any CPU
		Debug|Mixed Platforms = Debug|Mixed Platforms
		Debug|x86 = Debug|x86
		Release|Any CPU = Release|Any CPU
		Release|Mixed Platforms = Release|Mixed Platforms
		Release|x86 = Release|x86
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Mixed Platforms.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Mixed Platforms.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|x86.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|x86.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Any CPU.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Any CPU.Build.0 = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Mixed Platforms.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Mixed Platforms.Build.0 = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|x86.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|x86.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Any CPU.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Any CPU.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Mixed Platforms.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Mixed Platforms.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|x86.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|x86.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Any CPU.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Any CPU.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Mixed Platforms.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Mixed Platforms.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|x86.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|x86.Build.0 = Release|x86
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|x86.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Any CPU.Build.0 = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|x86.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|x86.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Any CPU.Build.0 = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|x86.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|x86.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Any CPU.Build.0 = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|x86.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|x86.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Any CPU.Build.0 = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|x86.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|x86.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Any CPU.Build.0 = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|x86.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|x86.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|x86.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Any CPU.Build.0 = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|x86.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|x86.Build.0 = Release|Any CPU
	EndGlobalSection
	GlobalSection(SolutionProperties) = preSolution
		HideSolutionNode = FALSE
	EndGlobalSection
EndGlobal
"#;
}
