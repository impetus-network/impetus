use frame_support::{assert_noop, assert_ok, derive_impl, traits::ConstU64};
use sp_core::{H160, U256};
use sp_runtime::BuildStorage;

use crate::{
	self as pallet_gasless_registry, Error, Event, GaslessDecision, Rules,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		GaslessRegistry: pallet_gasless_registry,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

impl pallet_gasless_registry::Config for Test {
	type ManageOrigin = frame_system::EnsureRoot<u64>;
	type MaxGaslessGasLimit = ConstU64<1_000_000>;
	type WeightInfo = ();
}

const CONTRACT: H160 = H160([0x11; 20]);
const OTHER_CONTRACT: H160 = H160([0x22; 20]);
const SELECTOR: [u8; 4] = [0xaa, 0xbb, 0xcc, 0xdd];

fn calldata() -> Vec<u8> {
	vec![0xaa, 0xbb, 0xcc, 0xdd, 0x01, 0x02]
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let storage = frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[test]
fn genesis_build_seeds_rules() {
	let mut storage = frame_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap();

	pallet_gasless_registry::GenesisConfig::<Test> {
		rules: vec![(CONTRACT, SELECTOR, U256::from(10), true)],
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		let rule = Rules::<Test>::get((CONTRACT, SELECTOR)).unwrap();
		assert!(rule.enabled);
		assert_eq!(rule.min_value, U256::from(10));
	});
}

#[test]
fn set_rule_stores_enabled_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::from(10),
			true,
		));

		let rule = Rules::<Test>::get((CONTRACT, SELECTOR)).unwrap();
		assert!(rule.enabled);
		assert_eq!(rule.min_value, U256::from(10));
		System::assert_last_event(RuntimeEvent::GaslessRegistry(Event::RuleSet {
			contract: CONTRACT,
			selector: SELECTOR,
			enabled: true,
			min_value: U256::from(10),
		}));
	});
}

#[test]
fn set_rule_accepts_zero_min_value_and_disabled_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::zero(),
			false,
		));

		let rule = Rules::<Test>::get((CONTRACT, SELECTOR)).unwrap();
		assert!(!rule.enabled);
		assert_eq!(rule.min_value, U256::zero());
	});
}

#[test]
fn set_rule_requires_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			GaslessRegistry::set_rule(
				RuntimeOrigin::signed(1),
				CONTRACT,
				SELECTOR,
				U256::zero(),
				true,
			),
			sp_runtime::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn remove_rule_deletes_existing_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::zero(),
			true,
		));

		assert_ok!(GaslessRegistry::remove_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
		));

		assert!(!Rules::<Test>::contains_key((CONTRACT, SELECTOR)));
		System::assert_last_event(RuntimeEvent::GaslessRegistry(Event::RuleRemoved {
			contract: CONTRACT,
			selector: SELECTOR,
		}));
	});
}

#[test]
fn remove_rule_rejects_missing_rule() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			GaslessRegistry::remove_rule(RuntimeOrigin::root(), CONTRACT, SELECTOR),
			Error::<Test>::RuleNotFound,
		);
	});
}

#[test]
fn evaluate_returns_gasless_for_matching_enabled_rule() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::from(10),
			true,
		));

		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &calldata(), U256::from(10), 500_000),
			GaslessDecision::Gasless,
		);
	});
}

#[test]
fn evaluate_returns_paid_for_non_matching_cases() {
	new_test_ext().execute_with(|| {
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			CONTRACT,
			SELECTOR,
			U256::from(10),
			true,
		));
		assert_ok!(GaslessRegistry::set_rule(
			RuntimeOrigin::root(),
			OTHER_CONTRACT,
			SELECTOR,
			U256::zero(),
			false,
		));

		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &[0xaa, 0xbb], U256::from(10), 500_000),
			GaslessDecision::Paid,
		);
		assert_eq!(
			GaslessRegistry::evaluate(OTHER_CONTRACT, &calldata(), U256::from(10), 500_000),
			GaslessDecision::Paid,
		);
		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &calldata(), U256::from(9), 500_000),
			GaslessDecision::Paid,
		);
		assert_eq!(
			GaslessRegistry::evaluate(CONTRACT, &calldata(), U256::from(10), 1_000_001),
			GaslessDecision::Paid,
		);
	});
}
