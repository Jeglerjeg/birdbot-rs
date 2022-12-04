use crate::Error;

pub fn remove_trailing_zeros(number: f64, precision: usize) -> Result<f64, Error> {
    Ok(
        (format!("{:.prec$}", number, prec = precision).parse::<f64>()? * 100_000_000.0).round()
            / 100_000_000.0,
    )
}
