//! The brain (design §5): what a sprite attends to and what it chooses to do.

use serde::Serialize;

use crate::data::DataPack;
use crate::expression::{Expression, expressions};
use crate::genome::{Gene, Genome};
use crate::registry::BrainParam;

/// The brain's parameters (design §5.7): each from the first `BrainParam`
/// gene that sets it, clamped to physiology's range, or physiology's
/// default where no gene does.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the brain reads its parameters later in slice 6")
)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct BrainParams {
    /// In `BrainParam::ALL` order.
    values: [f32; BrainParam::ALL.len()],
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the brain reads its parameters later in slice 6")
)]
impl BrainParams {
    pub(crate) fn express(genome: &Genome, data: &DataPack) -> BrainParams {
        let ranges = &data.physiology().brain;
        let mut values = BrainParam::ALL.map(|param| ranges.of(param).default);
        for (gene, expression) in genome.genes.iter().zip(expressions(genome, data)) {
            if let (&Gene::BrainParam { param, value }, Expression::Expressed) = (gene, expression)
            {
                let (low, high) = ranges.of(param).range;
                values[index(param)] = value.clamp(low, high);
            }
        }
        BrainParams { values }
    }

    /// The value of `param`.
    pub(crate) fn get(&self, param: BrainParam) -> f32 {
        self.values[index(param)]
    }
}

/// Where `param` is in `BrainParam::ALL`.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the brain reads its parameters later in slice 6")
)]
fn index(param: BrainParam) -> usize {
    BrainParam::ALL
        .iter()
        .position(|&p| p == param)
        .expect("every parameter is in ALL")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

    fn params(genes: &[&str]) -> BrainParams {
        let data = builtin();
        let text = format!("(format: 1, genes: [{}])", genes.join(", "));
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        BrainParams::express(&genome, &data)
    }

    #[test]
    fn a_brain_parameter_is_clamped_to_physiology_s_range() {
        let params = params(&[
            r#"BrainParam(param: "tau_base", value: 5.0)"#,
            r#"BrainParam(param: "switch_margin", value: 0.4)"#,
            r#"BrainParam(param: "switch_margin", value: 0.9)"#,
        ]);
        // physiology.ron: tau_base 0.05–2.0.
        assert_eq!(params.get(BrainParam::TauBase), 2.0);
        assert_eq!(
            params.get(BrainParam::SwitchMargin),
            0.4,
            "the first gene wins"
        );
    }

    #[test]
    fn a_brain_parameter_no_gene_sets_takes_physiology_s_default() {
        let params = params(&[]);
        // physiology.ron's defaults.
        assert_eq!(params.get(BrainParam::TauBase), 0.2);
        assert_eq!(params.get(BrainParam::SalienceGain), 0.5);
        assert_eq!(params.get(BrainParam::ForgetTicks), 5000.0);
    }
}
