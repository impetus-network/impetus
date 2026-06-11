//! Piecewise-linear inflation curve shared by every NPoS-enabled runtime.
//!
//! Matches Polkadot's defaults: 2.5%–10% inflation, ideal staking ratio 75%,
//! falloff 5%. The curve crate emits a `static REWARD_CURVE` symbol that
//! `pallet_staking::Config::EraPayout = ConvertCurve<RewardCurve>` consumes.

use pallet_staking_reward_curve::build;
use sp_runtime::curve::PiecewiseLinear;

build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_750_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

frame_support::parameter_types! {
    /// Public parameter-type wrapper around the generated curve.
    ///
    /// Use `RewardCurve::get()` wherever
    /// `pallet_staking::Config::EraPayout = ConvertCurve<RewardCurve>` is needed.
    pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
}
