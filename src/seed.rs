use rand::{RngCore, rngs::OsRng};

pub const GENERATION_SEED_ENV_VAR: &str = "COUNTRY_SIM_SEED";

pub fn parse_generation_seed(raw: &str) -> Option<u64> {
    let normalized = raw.trim().replace('_', "");
    if normalized.is_empty() {
        return None;
    }

    if let Some(hex) = normalized
        .strip_prefix("0x")
        .or_else(|| normalized.strip_prefix("0X"))
    {
        if hex.is_empty() {
            return None;
        }
        return u64::from_str_radix(hex, 16).ok();
    }

    normalized.parse::<u64>().ok()
}

pub fn format_generation_seed(seed: u64) -> String {
    format!("0x{seed:016x}")
}

pub fn random_generation_seed() -> u64 {
    OsRng.next_u64()
}

pub fn resolve_generation_seed(
    cli_seed: Option<&str>,
    env_seed: Option<&str>,
    random_fallback: u64,
) -> Result<u64, String> {
    if let Some(raw) = cli_seed {
        return parse_generation_seed(raw).ok_or_else(|| {
            format!("Invalid --seed value '{raw}'. Use decimal or 0x-prefixed hex.")
        });
    }

    if let Some(raw) = env_seed {
        return parse_generation_seed(raw).ok_or_else(|| {
            format!(
                "Invalid {} value '{raw}'. Use decimal or 0x-prefixed hex.",
                GENERATION_SEED_ENV_VAR
            )
        });
    }

    Ok(random_fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_generation_seed_accepts_decimal_hex_and_underscores() {
        assert_eq!(parse_generation_seed("42"), Some(42));
        assert_eq!(parse_generation_seed("0x2a"), Some(42));
        assert_eq!(parse_generation_seed("0X2A"), Some(42));
        assert_eq!(parse_generation_seed("0x5eed_c0de"), Some(0x5eed_c0de));
        assert_eq!(parse_generation_seed(" 1_000 "), Some(1_000));
    }

    #[test]
    fn parse_generation_seed_rejects_invalid_values() {
        assert_eq!(parse_generation_seed(""), None);
        assert_eq!(parse_generation_seed("0x"), None);
        assert_eq!(parse_generation_seed("abc"), None);
    }

    #[test]
    fn resolve_generation_seed_precedence_is_cli_then_env_then_random() {
        let resolved =
            resolve_generation_seed(Some("0x10"), Some("0x20"), 30).expect("cli should parse");
        assert_eq!(resolved, 16);

        let resolved = resolve_generation_seed(None, Some("0x20"), 30).expect("env should parse");
        assert_eq!(resolved, 32);

        let resolved =
            resolve_generation_seed(None, None, 30).expect("random fallback should be used");
        assert_eq!(resolved, 30);
    }

    #[test]
    fn resolve_generation_seed_errors_on_invalid_cli_or_env() {
        let err =
            resolve_generation_seed(Some("invalid"), Some("0x10"), 30).expect_err("cli invalid");
        assert!(err.contains("--seed"));

        let err = resolve_generation_seed(None, Some("invalid"), 30).expect_err("env invalid");
        assert!(err.contains(GENERATION_SEED_ENV_VAR));
    }
}
