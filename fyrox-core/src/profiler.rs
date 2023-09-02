//! Built-in scoped profiler. You must compile with feature "enable_profiler" to
//! force profiler gather info! It is disabled by default because it is not cheap
//! and takes 3-5% of performance for internal needs.

#![allow(dead_code)]

use fxhash::{FxHashMap, FxHashSet, FxHasher};
use std::{
    fmt,
    fmt::Write,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

pub fn print() -> Result<String, fmt::Error> {
    #[cfg(feature = "enable_profiler")]
    {
        let mut buffer = String::new();
        PROFILER.lock().unwrap().print(&mut buffer)?;
        Ok(buffer)
    }

    #[cfg(not(feature = "enable_profiler"))]
    {
        Ok("Performance profiling results are not available, because feature 'enable_profiler' wasn't defined!".to_owned())
    }
}

pub fn print_hot_path() -> Result<String, fmt::Error> {
    #[cfg(feature = "enable_profiler")]
    {
        let mut buffer = String::new();
        PROFILER.lock().unwrap().print_hot_path(&mut buffer)?;
        Ok(buffer)
    }

    #[cfg(not(feature = "enable_profiler"))]
    {
        Ok("Performance profiling results are not available, because feature 'enable_profiler' wasn't defined!".to_owned())
    }
}

struct Sample {
    count: u64,
    time: f64,
    children: FxHashSet<ScopeMark>,
}

impl Sample {
    pub fn collect(&mut self, time: f64) {
        self.time += time;
        self.count += 1;
    }
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            count: 0,
            time: 0.0,
            children: Default::default(),
        }
    }
}

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
struct ScopeMark {
    parent_scope_hash: u64,
    function_name: &'static str,
    line: u32,
}

struct Profiler {
    start_time: std::time::Instant,
    samples: FxHashMap<ScopeMark, Sample>,
    scope_stack: Vec<ScopeMark>,
}

const ENTRY_SCOPE_MARK: ScopeMark = ScopeMark {
    parent_scope_hash: 0,
    function_name: "EntryPoint",
    line: 0,
};

impl Default for Profiler {
    #[inline]
    fn default() -> Self {
        let entry_sample = Sample {
            count: 0,
            time: 0.0,
            children: FxHashSet::default(),
        };
        let mut samples = FxHashMap::default();
        samples.insert(ENTRY_SCOPE_MARK, entry_sample);
        Self {
            start_time: std::time::Instant::now(),
            samples,
            scope_stack: vec![ENTRY_SCOPE_MARK],
        }
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = FxHasher::default();
    t.hash(&mut s);
    s.finish()
}

impl Profiler {
    fn enter_scope(&mut self, scope: &mut ScopeMark) {
        let parent_scope_mark = *self.scope_stack.last().unwrap();
        scope.parent_scope_hash = calculate_hash(&parent_scope_mark);
        self.scope_stack.push(*scope);
        self.samples.entry(*scope).or_default();
        self.samples
            .get_mut(&parent_scope_mark)
            .unwrap()
            .children
            .insert(*scope);
    }

    fn leave_scope(&mut self, scope: ScopeMark, elapsed: f64) {
        self.scope_stack.pop();
        self.samples.get_mut(&scope).unwrap().collect(elapsed);
    }

    fn print(&self, buffer: &mut String) -> fmt::Result {
        let full_time = (std::time::Instant::now() - self.start_time).as_secs_f64();
        self.recursive_print(buffer, &ENTRY_SCOPE_MARK, 0, full_time)?;
        writeln!(buffer,"=========================================================================================================")?;
        Ok(())
    }

    fn recursive_print(
        &self,
        buffer: &mut String,
        scope_mark: &ScopeMark,
        offset: usize,
        full_time: f64,
    ) -> fmt::Result {
        let sample = self.samples.get(scope_mark).unwrap();

        if scope_mark == &ENTRY_SCOPE_MARK {
            writeln!(buffer, "=========================================================================================================")?;
            writeln!(buffer,"Profiling took {} seconds. Please note that profiling itself takes time so results are not 100% accurate.", full_time)?;
            writeln!(buffer, "Entry Point")?;
        } else {
            writeln!(
                buffer,
                "{}{:.4}% - {} at line {} was called {} times and took {} seconds individually.",
                "\t".repeat(offset),
                (sample.time / full_time) * 100.0,
                scope_mark.function_name,
                scope_mark.line,
                sample.count,
                sample.time
            )?;
        }

        for child_scope in self
            .samples
            .get(scope_mark)
            .as_ref()
            .unwrap()
            .children
            .iter()
        {
            self.recursive_print(buffer, child_scope, offset + 1, full_time)?;
        }

        Ok(())
    }

    fn print_hot_path(&self, buffer: &mut String) -> fmt::Result {
        let full_time = (std::time::Instant::now() - self.start_time).as_secs_f64();
        self.print_hot_path_recursive(buffer, &ENTRY_SCOPE_MARK, 0, full_time)?;
        writeln!(buffer, "=========================================================================================================")?;
        Ok(())
    }

    fn print_hot_path_recursive(
        &self,
        buffer: &mut String,
        scope_mark: &ScopeMark,
        offset: usize,
        full_time: f64,
    ) -> fmt::Result {
        let sample = self.samples.get(scope_mark).unwrap();

        if scope_mark == &ENTRY_SCOPE_MARK {
            writeln!(buffer,"=========================================================================================================")?;
            writeln!(buffer,"Showing hot path only! Profiling took {} seconds. Please note that profiling itself takes time so results are not 100% accurate.", full_time)?;
            writeln!(buffer, "Entry Point")?;
        } else {
            writeln!(
                buffer,
                "{}{:.4}% - {} at line {} was called {} times and took {} seconds individually.",
                "\t".repeat(offset),
                (sample.time / full_time) * 100.0,
                scope_mark.function_name,
                scope_mark.line,
                sample.count,
                sample.time
            )?;
        }

        let mut hot = None;
        let mut hot_time = 0.0;
        for child_scope in self
            .samples
            .get(scope_mark)
            .as_ref()
            .unwrap()
            .children
            .iter()
        {
            let time = self.samples.get(child_scope).as_ref().unwrap().time;
            if time > hot_time {
                hot_time = time;
                hot = Some(*child_scope);
            }
        }

        if let Some(hot) = hot.as_ref() {
            self.print_hot_path_recursive(buffer, hot, offset + 1, full_time)?;
        }

        Ok(())
    }
}

lazy_static! {
    static ref PROFILER: Arc<Mutex<Profiler>> = Arc::new(Mutex::new(Profiler::default()));
}

pub struct ScopeDefinition {
    scope: ScopeMark,
    start_time: std::time::Instant,
}

impl ScopeDefinition {
    #[cfg(feature = "enable_profiler")]
    #[inline]
    pub fn new(function_name: &'static str, line: u32) -> Self {
        let mut scope = ScopeMark {
            parent_scope_hash: 0,
            function_name,
            line,
        };

        PROFILER.lock().unwrap().enter_scope(&mut scope);

        Self {
            scope,
            start_time: std::time::Instant::now(),
        }
    }

    #[inline]
    fn elapsed(&self) -> f64 {
        (std::time::Instant::now() - self.start_time).as_secs_f64()
    }
}

impl Drop for ScopeDefinition {
    fn drop(&mut self) {
        let elapsed = self.elapsed();
        PROFILER.lock().unwrap().leave_scope(self.scope, elapsed);
    }
}

#[inline]
pub fn type_name_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

#[cfg(feature = "enable_profiler")]
#[macro_export]
macro_rules! scope_profile {
    () => {
        let function_name = {
            fn scope() {}
            $crate::profiler::type_name_of(scope)
        };
        let _scope_guard = $crate::profiler::ScopeDefinition::new(function_name, line!());
    };
}

#[cfg(not(feature = "enable_profiler"))]
#[macro_export]
macro_rules! scope_profile {
    () => {};
}

#[cfg(test)]
mod test {
    use crate::profiler;
    use std::time::Duration;

    use super::*;

    fn nested_func() {
        scope_profile!();
        std::thread::sleep(Duration::from_millis(1000));
    }

    fn some_func() {
        scope_profile!();
        nested_func();
    }

    fn other_func() {
        scope_profile!();
        std::thread::sleep(Duration::from_millis(1000));
        nested_func();
        some_func();
    }

    #[test]
    fn test_scope_perf() {
        {
            scope_profile!();
            other_func();
        }

        println!("{:?}", profiler::print());
    }

    #[test]
    fn default_for_sample() {
        let s = Sample::default();

        assert_eq!(s.time, 0.0);
        assert_eq!(s.count, 0);
        assert_eq!(s.children, Default::default());
    }

    #[test]
    fn sample_collect() {
        let mut s = Sample::default();
        s.collect(5.0);
        s.collect(2.0);

        assert_eq!(s.time, 7.0);
        assert_eq!(s.count, 2);
        assert_eq!(s.children, Default::default());
    }

    #[test]
    fn default_for_profiler() {
        let profiler = Profiler::default();
        let sample = Sample::default();

        assert_eq!(profiler.samples.len(), 1);
        assert!(profiler.samples.contains_key(&ENTRY_SCOPE_MARK));

        let v = profiler.samples.get(&ENTRY_SCOPE_MARK).unwrap();
        assert_eq!(v.time, sample.time);
        assert_eq!(v.count, sample.count);
        assert_eq!(v.children, sample.children);

        assert_eq!(profiler.scope_stack, [ENTRY_SCOPE_MARK]);
    }

    #[test]
    fn profiler_enter_scope() {
        let mut mark = ScopeMark {
            parent_scope_hash: 0,
            function_name: "foo",
            line: 0,
        };
        let mut profiler = Profiler::default();
        profiler.enter_scope(&mut mark);

        assert_eq!(profiler.samples.len(), 2);
        assert_eq!(profiler.scope_stack, [ENTRY_SCOPE_MARK, mark]);
        assert_eq!(mark.parent_scope_hash, calculate_hash(&ENTRY_SCOPE_MARK))
    }

    #[test]
    fn profiler_leave_scope() {
        let mut mark = ScopeMark {
            parent_scope_hash: 0,
            function_name: "foo",
            line: 0,
        };
        let mut profiler = Profiler::default();
        profiler.enter_scope(&mut mark);
        profiler.leave_scope(mark, 42.0);

        assert_eq!(profiler.scope_stack, [ENTRY_SCOPE_MARK]);

        let v = profiler.samples.get(&mark).unwrap();
        assert_eq!(v.time, 42.0);
        assert_eq!(v.count, 1);
    }

    #[test]
    fn profiler_print() {
        let mut profiler = Profiler::default();
        let mut buffer = String::default();
        let mut mark = ScopeMark {
            parent_scope_hash: 0,
            function_name: "foo",
            line: 0,
        };
        profiler.enter_scope(&mut mark);

        scope_profile!();
        std::thread::sleep(Duration::from_millis(1000));

        let res = profiler.print(&mut buffer);
        assert!(res.is_ok());
        assert!(buffer.starts_with("========================================================================================================="));
        assert!(buffer.contains(
"seconds. Please note that profiling itself takes time so results are not 100% accurate.\nEntry Point\n\t0.0000% - foo at line 0 was called 0 times and took 0 seconds individually.\n=========================================================================================================\n") );
    }

    #[test]
    fn profiler_print_hot_path() {
        let profiler = Profiler::default();
        let mut buffer = String::default();

        scope_profile!();
        std::thread::sleep(Duration::from_millis(1000));

        let res = profiler.print_hot_path(&mut buffer);
        assert!(res.is_ok());
        assert!(buffer.starts_with("========================================================================================================="));
        assert!(buffer.contains(
r#"seconds. Please note that profiling itself takes time so results are not 100% accurate.
Entry Point
========================================================================================================="#) );
    }

    #[test]
    fn test_type_name_of() {
        assert_eq!(type_name_of(42), "i32");
    }
}
