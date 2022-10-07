pub fn remove_trailing_zeros(number: f64, precision: usize) -> f64 {
    (format!("{:.prec$}", number, prec = precision)
        .parse::<f64>()
        .unwrap()
        * 100_000_000.0)
        .round()
        / 100_000_000.0
}
