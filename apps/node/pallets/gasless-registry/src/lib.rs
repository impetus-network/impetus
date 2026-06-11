#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	extern crate alloc;

	use alloc::vec::Vec;
	use frame_support::{pallet_prelude::*, traits::EnsureOrigin, weights::constants::RocksDbWeight};
	use frame_system::pallet_prelude::*;
	use sp_core::{H160, U256};


	#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, MaxEncodedLen, TypeInfo)]
	pub struct Rule {
		pub enabled: bool,
		pub min_value: U256,
	}

	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
	pub enum GaslessDecision {
		Gasless,
		Paid,
	}

	impl GaslessDecision {
		pub fn is_gasless(self) -> bool {
			matches!(self, Self::Gasless)
		}
	}

	pub trait WeightInfo {
		fn set_rule() -> Weight;
		fn remove_rule() -> Weight;
		fn evaluate() -> Weight;
	}

	impl WeightInfo for () {
		fn set_rule() -> Weight {
			Weight::from_parts(10_000, 0).saturating_add(RocksDbWeight::get().writes(1))
		}

		fn remove_rule() -> Weight {
			Weight::from_parts(10_000, 0)
				.saturating_add(RocksDbWeight::get().reads(1))
				.saturating_add(RocksDbWeight::get().writes(1))
		}

		fn evaluate() -> Weight {
			Weight::from_parts(10_000, 0).saturating_add(RocksDbWeight::get().reads(1))
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
		type ManageOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		#[pallet::constant]
		type MaxGaslessGasLimit: Get<u64>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	pub type Rules<T: Config> =
		StorageMap<_, Blake2_128Concat, (H160, [u8; 4]), Rule>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RuleSet {
			contract: H160,
			selector: [u8; 4],
			enabled: bool,
			min_value: U256,
		},
		RuleRemoved {
			contract: H160,
			selector: [u8; 4],
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		RuleNotFound,
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub rules: Vec<(H160, [u8; 4], U256, bool)>,
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (contract, selector, min_value, enabled) in &self.rules {
				Rules::<T>::insert(
					(*contract, *selector),
					Rule {
						enabled: *enabled,
						min_value: *min_value,
					},
				);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn evaluate(
			contract: H160,
			calldata: &[u8],
			value: U256,
			gas_limit: u64,
		) -> GaslessDecision {
			if calldata.len() < 4 {
				return GaslessDecision::Paid;
			}

			if gas_limit > T::MaxGaslessGasLimit::get() {
				return GaslessDecision::Paid;
			}

			let selector = [calldata[0], calldata[1], calldata[2], calldata[3]];
			match Rules::<T>::get((contract, selector)) {
				Some(rule) if rule.enabled && value >= rule.min_value => {
					GaslessDecision::Gasless
				}
				_ => GaslessDecision::Paid,
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_rule())]
		pub fn set_rule(
			origin: OriginFor<T>,
			contract: H160,
			selector: [u8; 4],
			min_value: U256,
			enabled: bool,
		) -> DispatchResult {
			T::ManageOrigin::ensure_origin(origin)?;

			Rules::<T>::insert((contract, selector), Rule { enabled, min_value });
			Self::deposit_event(Event::RuleSet {
				contract,
				selector,
				enabled,
				min_value,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::remove_rule())]
		pub fn remove_rule(
			origin: OriginFor<T>,
			contract: H160,
			selector: [u8; 4],
		) -> DispatchResult {
			T::ManageOrigin::ensure_origin(origin)?;
			ensure!(
				Rules::<T>::contains_key((contract, selector)),
				Error::<T>::RuleNotFound
			);

			Rules::<T>::remove((contract, selector));
			Self::deposit_event(Event::RuleRemoved { contract, selector });

			Ok(())
		}
	}
}
