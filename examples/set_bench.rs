extern crate bellman_bignat;
extern crate num_bigint;
extern crate rand;
extern crate sapling_crypto;
extern crate docopt;
extern crate serde;

use bellman_bignat::bignat::nat_to_limbs;
use bellman_bignat::group::RsaGroup;
use bellman_bignat::set::{SetBench, SetBenchInputs, SetBenchParams};
use docopt::Docopt;
use serde::Deserialize;
use num_bigint::BigUint;
use sapling_crypto::poseidon::bn256::Bn256PoseidonParams;
use sapling_crypto::bellman::pairing::ff::ScalarEngine;
use sapling_crypto::bellman::pairing::bn256::Bn256;
use sapling_crypto::bellman::Circuit;
use sapling_crypto::circuit::test::TestConstraintSystem;

use std::rc::Rc;
use std::str::FromStr;

const USAGE: &str = "
Set Benchmarker

Usage:
  set_bench rsa <transactions> <capacity>
  set_bench (-h | --help)
  set_bench --version

Options:
  -h --help     Show this screen.
  --version     Show version.
";

// From https://en.wikipedia.org/wiki/RSA_numbers#RSA-2048
const RSA_2048: &str = "25195908475657893494027183240048398571429282126204032027777137836043662020707595556264018525880784406918290641249515082189298559149176184502808489120072844992687392807287776735971418347270261896375014971824691165077613379859095700097330459748808428401797429100642458691817195118746121515172654632282216869987549182422433637259085141865462043576798423387184774447920739934236584823824281198163815010674810451660377306056201619676256133844143603833904414952634432190114657544454178424020924616515723350778707749817125772467962926386356373289912154831438167899885040445364023527381951378636564391212010397122822120720357";
const RSA_SIZE: usize = 2048;
const ELEMENT_SIZE: usize = 5;

#[derive(Debug, Deserialize)]
struct Args {
    arg_transactions: usize,
    arg_capacity: usize,
    cmd_rsa: bool,
}


fn main() {
    color_backtrace::install();
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    println!("{:?}", args);

    let hash = Rc::new(Bn256PoseidonParams::new::<
        sapling_crypto::group_hash::Keccak256Hasher,
    >());

    let group = RsaGroup {
        g: BigUint::from(2usize),
        m: BigUint::from_str(RSA_2048).unwrap(),
    };

    let circuit = SetBench {
        inputs: Some(SetBenchInputs::from_counts(
            0,
            args.arg_transactions,
            args.arg_transactions,
            ELEMENT_SIZE,
            hash.clone(),
            RSA_SIZE,
            32,
            RsaGroup {
                g: BigUint::from(2usize),
                m: BigUint::from_str(RSA_2048).unwrap(),
            },
        )),
        params: SetBenchParams {
            group: group.clone(),
            limb_width: 32,
            n_bits_elem: RSA_SIZE,
            n_bits_challenge: 128,
            n_bits_base: RSA_SIZE,
            item_size: ELEMENT_SIZE,
            n_inserts: args.arg_transactions,
            n_removes: args.arg_transactions,
            hash,
        },
    };

    let ins = circuit.inputs.as_ref().unwrap();
    let initial_set = ins.initial_state.clone();
    let final_set = {
        let mut t = initial_set.clone();
        t.remove_all(ins.to_remove.iter().map(Vec::as_slice));
        t.insert_all(ins.to_insert.clone());
        t
    };

    let mut inputs: Vec<<Bn256 as ScalarEngine>::Fr> = nat_to_limbs(&group.g, 32, 64).unwrap();
    inputs.extend(nat_to_limbs::<<Bn256 as ScalarEngine>::Fr>(
        &group.m, 32, 64,
    ).unwrap());
    inputs.extend(nat_to_limbs::<<Bn256 as ScalarEngine>::Fr>(
        &initial_set.digest(),
        32,
        64,
    ).unwrap());
    inputs.extend(nat_to_limbs::<<Bn256 as ScalarEngine>::Fr>(
        &final_set.digest(),
        32,
        64,
    ).unwrap());

    let mut cs = TestConstraintSystem::<Bn256>::new();
    circuit.synthesize(&mut cs).expect("synthesis failed");
    assert!(cs.is_satisfied());
    println!("Constraints: {}", cs.num_constraints());

}