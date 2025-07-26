pub(crate) fn shortest_unique_prefixes(values: &[&str]) -> (usize, Vec<String>, Vec<String>) {
    if values.is_empty() {
        return (0, Vec::new(), Vec::new());
    }

    let len = values[0].len();
    let mut global_prefix_len = 0;
    let mut individual_prefixes = Vec::with_capacity(values.len());

    // Helper to find shared prefix length
    fn shared_prefix_len(a: &str, b: &str) -> usize {
        a.chars()
            .zip(b.chars())
            .take_while(|(ac, bc)| ac == bc)
            .count()
    }

    for i in 0..values.len() {
        let cur = values[i];
        let mut max_shared = 0;

        if i > 0 {
            max_shared = max_shared.max(shared_prefix_len(cur, values[i - 1]));
        }
        if i + 1 < values.len() {
            max_shared = max_shared.max(shared_prefix_len(cur, values[i + 1]));
        }

        // Add 1 to ensure uniqueness
        let unique_len = (max_shared + 1).min(len);
        individual_prefixes.push(cur[..unique_len].to_string());

        // For global prefix: max shared between any two neighbors
        if i + 1 < values.len() {
            global_prefix_len = global_prefix_len.max(shared_prefix_len(cur, values[i + 1]) + 1);
        }
    }

    global_prefix_len = global_prefix_len.min(len);
    let global_prefixes = values
        .iter()
        .map(|s| s[..global_prefix_len].to_string())
        .collect();

    (global_prefix_len, global_prefixes, individual_prefixes)
}

#[cfg(test)]
pub(crate) mod test {
    use crate::find_prefix::shortest_unique_prefixes;

    #[test]
    fn simple_shortest() {
        let mut input = vec!["1ab", "3ab", "1ca"];
        let expected_len: usize = 2;
        let expected_global: Vec<String> = vec!["1a".into(), "1c".into(), "3a".into()];
        let expected_individual: Vec<String> = vec!["1a".into(), "1c".into(), "3".into()];

        input.sort();

        let (len, global_prefixes, individual_prefixes) = shortest_unique_prefixes(&input);

        assert_eq!(expected_len, len);
        assert_eq!(expected_global, global_prefixes);
        assert_eq!(expected_individual, individual_prefixes);
    }
}
