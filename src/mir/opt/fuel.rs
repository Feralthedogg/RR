use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FuelExhaustedReason {
    PassEntry,
    Work,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FuelBudget {
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OptimizationFuel {
    remaining: usize,
    consumed: usize,
    exhausted: bool,
}

impl OptimizationFuel {
    pub(crate) fn disabled() -> Self {
        Self {
            remaining: usize::MAX / 4,
            consumed: 0,
            exhausted: false,
        }
    }

    pub(crate) fn with_budget(budget: FuelBudget) -> Self {
        Self {
            remaining: budget.limit,
            consumed: 0,
            exhausted: false,
        }
    }

    pub(crate) fn consume(&mut self, amount: usize) -> Result<(), FuelExhaustedReason> {
        if self.exhausted {
            return Err(FuelExhaustedReason::Work);
        }
        let amount = amount.max(1);
        if self.remaining < amount {
            self.consumed = self.consumed.saturating_add(self.remaining);
            self.remaining = 0;
            self.exhausted = true;
            return Err(FuelExhaustedReason::Work);
        }
        self.remaining -= amount;
        self.consumed = self.consumed.saturating_add(amount);
        Ok(())
    }

    pub(crate) fn consume_pass_entry(&mut self, amount: usize) -> Result<(), FuelExhaustedReason> {
        self.consume(amount)
            .map_err(|_| FuelExhaustedReason::PassEntry)
    }

    pub(crate) const fn consumed(self) -> usize {
        self.consumed
    }

    pub(crate) const fn exhausted(self) -> bool {
        self.exhausted
    }
}

impl TachyonEngine {
    pub(crate) fn opt_fuel_enabled() -> bool {
        !matches!(
            std::env::var("RR_OPT_FUEL")
                .unwrap_or_else(|_| "1".to_string())
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "0" | "false" | "no" | "off"
        )
    }

    pub(crate) fn opt_fuel_trace_enabled() -> bool {
        Self::env_bool("RR_OPT_FUEL_TRACE", false)
    }

    pub(crate) fn function_fuel_budget(&self, ir_size: usize) -> FuelBudget {
        if !Self::opt_fuel_enabled() {
            return FuelBudget {
                limit: usize::MAX / 4,
            };
        }
        let default = if self.fast_dev_enabled() {
            ir_size.saturating_mul(120).clamp(10_000, 120_000)
        } else if self.aggressive_opt_enabled() {
            ir_size.saturating_mul(800).clamp(120_000, 2_000_000)
        } else {
            ir_size.saturating_mul(400).clamp(50_000, 500_000)
        };
        FuelBudget {
            limit: Self::env_usize("RR_OPT_FUEL", default),
        }
    }

    pub(crate) fn fuel_for_function(&self, ir_size: usize) -> OptimizationFuel {
        if Self::opt_fuel_enabled() {
            OptimizationFuel::with_budget(self.function_fuel_budget(ir_size))
        } else {
            OptimizationFuel::disabled()
        }
    }
}
