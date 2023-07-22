use anyhow::*;
use glob::glob;
use lcs;
use rayon::{
    prelude::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    *,
};
use regex::Regex;
use std::{collections::HashMap, io::Lines, path::PathBuf, sync::Mutex};

#[derive(Clone, Debug)]
pub struct LogMatch {
    pub file: String,
    pub line: usize,
    pub score: usize,
}

#[derive(Clone, Debug)]
pub struct LogLineSearch {
    pub message: String,
    pub func: Option<String>,
    pub file: Option<String>,
    pub line: Option<usize>,
}

impl LogLineSearch {
    pub fn new(pattern: &Regex, log_line: &str) -> Result<Self> {
        let captures = pattern
            .captures(&log_line)
            .context(format!("Regex did not match line: {}", log_line))?;
        Ok(LogLineSearch {
            message: captures
                .name("message")
                .context("Could not find message")?
                .as_str()
                .to_string()
                .trim()
                .to_string(),
            func: captures
                .name("func")
                .map(|m| m.as_str().to_string().trim().to_string()),
            file: captures
                .name("file")
                .map(|m| m.as_str().to_string().trim().to_string()),
            line: captures
                .name("line")
                .map(|m| m.as_str().to_string().parse().unwrap()),
        })
    }
}

/*
This is the search parameter we use.
we split the two strings up into "tokens".
We split on any non-alphanumeric char, but keep symbols as distinct tokens.
We then compare the number of chars in the longest common subsequence of tokens.
E.g
needle: This is a log message: 0xbeef
haystack: log!("This is a log message: {:x}", beef_variable);
Would get us the LCS: ["This", "is", "a", "log", "message", ":"]
Giving us a score of     4   +   2 +  1  +  3  +  7 + 1 = 18
We don't normalize, as we are only comparing result using the same needle.
*/
fn token_lcs(haystack: &str, needle: &str) -> usize {
    fn strip_unicode(a: &str) -> String {
        a.chars().filter(|c| c.is_ascii()).collect()
    }
    let haystack = strip_unicode(haystack);
    let needle = strip_unicode(needle);

    let haystack_seq: Vec<_> = haystack
        .split(|c: char| !c.is_alphanumeric())
        .filter(|c| !c.is_empty())
        .collect();
    let needle_seq: Vec<_> = needle
        .split(|c: char| !c.is_alphanumeric())
        .filter(|c| !c.is_empty())
        .collect();
    let table = lcs::LcsTable::new(&haystack_seq, &needle_seq);
    let lcs_res = table.longest_common_subsequence();

    lcs_res.into_iter().map(|(a, _b)| a.len()).sum::<usize>()
}

fn best_match_in_file(
    contents: &str,
    filename: &str,
    search_options: &LogLineSearch,
) -> Option<LogMatch> {
    if let Some(ref func) = search_options.func {
        if !contents.contains(func) {
            return None;
        }
    }
    let result = match search_options.line {
        None => {
            let lines = contents.lines();
            let message: &str = &search_options.message;
            let scores = lines
                .enumerate()
                .map(|(i, l)| (i + 1, token_lcs(l, message)));

            if let Some(best) = scores.max_by(|a, b| a.1.cmp(&b.1)) {
                return Some(LogMatch {
                    file: filename.to_string(),
                    line: best.0,
                    score: best.1,
                });
            }

            None
        }
        Some(line) => {
            let l = contents.lines().nth(line - 1)?;
            let b = Some(LogMatch {
                file: filename.to_string(),
                line: line,
                score: token_lcs(l, &search_options.message),
            });

            b
        }
    };

    result
}

#[derive(Clone, Debug)]
pub struct LogSearchSettings {
    pub log_file_name: String,
    pub log_file: String,
    pub log_pattern: Regex,
    pub include: String,
}

use lazy_static::lazy_static;
use moka;
use moka::{sync::Cache, Entry};
lazy_static! {
    static ref SEARCH_CACHE: Cache<String, Option<LogMatch>> = Cache::new(10_000);
}

pub fn search_files(
    files: &Vec<(String, String)>,
    search_params: &LogSearchSettings,
    log_line: &str,
) -> Option<LogMatch> {
    let cache = SEARCH_CACHE.clone();

    #[cfg(feature = "test-server")]
    cache.invalidate_all();

    cache.get_with(log_line.to_string(), || {
        let search_options = LogLineSearch::new(&search_params.log_pattern, log_line).ok()?;

        let matches = files
            .iter()
            .par_bridge()
            .filter(|(f, c)| {
                if let Some(file) = &search_options.file {
                    return f.as_str() == file;
                }
                true
            })
            .flat_map(|(f, c)| best_match_in_file(&c, &f, &search_options));

        matches.max_by(|a, b| a.score.cmp(&b.score))
    })
}
