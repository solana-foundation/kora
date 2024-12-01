use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub enum Feature {
    Gasless,
}

impl FromStr for Feature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gasless" => Ok(Feature::Gasless),
            _ => Err(format!("Unknown feature: {}", s)),
        }
    }
}