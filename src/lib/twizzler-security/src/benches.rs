// i cant make it stop complaining
#![allow(unused_imports)]

extern crate test;
use test::Bencher;
use twizzler::object::TypedObject;
use twizzler_abi::{
    object::Protections,
    syscall::{LifetimeType, ObjectCreate, ObjectCreateFlags},
};

use crate::*;

#[bench]
fn capability_creation(b: &mut Bencher) {
    let (s, _v) = SigningKey::new_keypair(&SigningScheme::Ecdsa, ObjectCreate::default())
        .expect("keypair creation should not have errored!");

    b.iter(|| {
        let _cap = Cap::new(
            0x123.into(),
            0x321.into(),
            Protections::all(),
            s.base(),
            Revoc::default(),
            Gates::default(),
            HashingAlgo::Sha256,
        )
        .expect("Capability should have been created.");
    })
}

#[bench]
//WARN: passing in the LifetimeType as Persistent will cause the test to just hang permanently
fn keypair_creation(b: &mut Bencher) {
    let object_create_spec = ObjectCreate::new(
        Default::default(),
        LifetimeType::Volatile,
        Default::default(),
        ObjectCreateFlags::empty(),
        Protections::all(),
    );

    b.iter(|| {
        println!("running!");
        let _ = core::hint::black_box(
            SigningKey::new_keypair(&SigningScheme::Ecdsa, object_create_spec)
                .expect("Keys should be generated properly"),
        );
    });
}

#[bench]
fn capability_verification(b: &mut Bencher) {
    let (s, v) = SigningKey::new_keypair(&SigningScheme::Ecdsa, ObjectCreate::default())
        .expect("keypair creation should not have errored!");

    let cap = Cap::new(
        0x123.into(),
        0x321.into(),
        Protections::all(),
        s.base(),
        Revoc::default(),
        Gates::default(),
        HashingAlgo::Sha256,
    )
    .expect("Capability should have been created.");

    b.iter(|| {
        cap.verify_sig(v.base())
            .expect("capability should have been verified.");
    })
}
