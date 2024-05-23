use std::ops::Mul;
use elliptic_curve::{AffinePoint, CurveArithmetic, Field, Group, PrimeCurveArithmetic, PrimeField, ProjectivePoint, Scalar, ScalarPrimitive};
use elliptic_curve::bigint::{AddMod, CheckedAdd, CheckedMul};
use elliptic_curve::group::Curve;
use elliptic_curve::point::{AffineCoordinates, PointCompression};
use elliptic_curve::rand_core::CryptoRngCore;
use elliptic_curve::sec1::{FromEncodedPoint, ModulusSize, ToEncodedPoint};
use digest::{FixedOutput, HashMarker, Update, Digest};

use crate::crypto::schnorr::Shnorr;

fn commitment<Curve: CurveArithmetic>(rng: &mut impl CryptoRngCore) -> (Scalar<Curve>, AffinePoint<Curve>) {
    let nonce = Scalar::<Curve>::random(rng);
    let commitment = ProjectivePoint::<Curve>::generator() * nonce;
    (nonce, commitment.into())
}

fn challenge<Curve, Hasher, T>(public_key: &AffinePoint<Curve>, payload: &T) -> Scalar<Curve>
where
    Curve: CurveArithmetic + PointCompression,
    <Curve as CurveArithmetic>::AffinePoint: FromEncodedPoint<Curve> + ToEncodedPoint<Curve>,
    <Curve as elliptic_curve::Curve>::FieldBytesSize: ModulusSize,
    Hasher: FixedOutput + Default + Update + HashMarker,
    T: AsRef<[u8]>,
{
    let hash = Hasher::default()
        .chain(public_key.to_encoded_point(true).as_bytes())
        .chain(payload.as_ref())
        .finalize();
    let a = ScalarPrimitive::<Curve>::from_slice(hash.as_ref()).unwrap();
    a.into()
}

impl<Curve> Shnorr<Scalar<Curve>, AffinePoint<Curve>> for Curve
where
    Curve: CurveArithmetic + PointCompression,
    <Curve as CurveArithmetic>::AffinePoint: FromEncodedPoint<Curve> + ToEncodedPoint<Curve>,
    <Curve as elliptic_curve::Curve>::FieldBytesSize: ModulusSize
{
    fn proof<T, Hasher>(&self, payload: &T, x: &Scalar<Curve>, rng: &mut impl CryptoRngCore) -> (Scalar<Curve>, AffinePoint<Curve>)
    where
        T: AsRef<[u8]>,
        Hasher: FixedOutput + Default + Update + HashMarker,
    {
        let (c, commitment) = commitment::<Curve>(rng);
        let challenge = challenge::<Curve, Hasher, T>(&commitment.into(), payload);
        let proof = c + x.mul(&challenge);
        (proof, commitment)
    }

    fn verify<T, Hasher>(&self, payload: &T, public_key: &AffinePoint<Curve>, proof: &Scalar<Curve>, commitment: &AffinePoint<Curve>) -> bool
    where
        T: AsRef<[u8]>,
        Hasher: FixedOutput + Default + Update + HashMarker,
    {
        let challenge = challenge::<Curve, Hasher, T>(commitment.into(), payload);
        let lhs = ProjectivePoint::<Curve>::generator() * proof;
        let commitment = ProjectivePoint::<Curve>::from(*commitment);
        let public_key = ProjectivePoint::<Curve>::from(*public_key);
        let rhs = commitment + public_key.mul(&challenge);
        lhs == rhs
    }
}

#[cfg(test)]
mod tests {
    use p256::NistP256;
    use sha3::digest::core_api::CoreWrapper;
    use sha3::Sha3_256Core;
    use crate::crypto::csprng::ChaChaRng;
    use crate::crypto::elliptic_curve::{commitment, Shnorr};

    #[test]
    fn test_zkp() {
        let mut rng = ChaChaRng::new();
        let (private_key, public_key) = commitment::<NistP256>(&mut rng);
        let (proof, commitment) = NistP256.proof::<_, CoreWrapper<Sha3_256Core>>(
            b"hello world",
            &private_key,
            &mut rng
        );
        assert!(NistP256.verify::<_, CoreWrapper<Sha3_256Core>>(
            b"hello world",
            &public_key,
            &proof,
            &commitment
        ));
    }
}
