use std::marker::PhantomData;
use halo2_proofs::{
    arithmetic::FieldExt,
    // circuit::*,
    pasta::*,
    plonk::*,
    poly::Rotation, circuit::{SimpleFloorPlanner, AssignedCell, Layouter},
};

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct FiboConfig {
    pub col_a: Column<Advice>,
    pub col_b: Column<Advice>,
    pub col_c: Column<Advice>,
    pub selector: Selector,
}

#[derive(Debug, Clone)]
struct FiboChip<F: FieldExt> {
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FiboChip<F> {
    fn construct(config: FiboConfig) -> Self {
        Self { config, _marker: PhantomData }

    }

    fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: [Column<Advice>; 3],
    ) -> FiboConfig {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let selector = meta.selector();


        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);

        meta.create_gate("add",  |meta|{
            let s: Expression<F> = meta.query_selector(selector);
            let a: Expression<F> = meta.query_advice(col_a, Rotation::cur());
            let b: Expression<F> = meta.query_advice(col_b, Rotation::cur());
            let c: Expression<F> = meta.query_advice(col_c, Rotation::cur());
            vec![s * (a+b-c)];
        });

        FiboConfig {
            col_a, 
            col_b, 
            col_c,
            selector,
        }
    }

    fn assign_first_row(&self, mut layouter: impl Layouter<F>, a: Option<F>, b: Option<F>)
    -> Result<(ACell<F>, ACell<F>, ACell<F>), Error>{
        layouter.assign_region(
            || "first row",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                let a_cell = region.assign_advice(
                    || "a",
                    self.config.col_a,
                    0, 
                    || a.ok_or(Error::Synthesis),
                ).map(ACell)?;


                let b_cell = region.assign_advice(
                    || "b",
                    self.config.col_b,
                    0, 
                    || b.ok_or(Error::Synthesis),
                ).map(ACell)?;

                let c_val = a.and_then(|a| b.map(|b| a + b));

                let c_cell = region.assign_advice(
                    || "c",
                    self.config.col_c,
                    0,
                    || c_val.ok_or(Error::Synthesis),
                )?;

                Ok((a_cell, b_cell, c_cell))
            })

    }

    fn assign_row(&self, mut layouter: impl Layouter<F>, prev_b: &ACell<F>, prev_c: &ACell<F>)
        -> Result<ACell<F>, Error> {
            layouter.assign_region(
                || "next row",
                |mut region: Region<{unknown}>| {
                    self.config.selector.enable(&mut region, 0);
                    prev_b.0.copy_advice(|| "a", &mut region, self.config.col_a, 0)?;
                    prev_c.0.copy_advice(|| "b", &mut region, self.config.col_b, 0)?;

                    let c_val = prev_b.0.value().and_then(
                        |b| {
                            prev_c.0.value().map(|c| *b + *c)
                        }
                    );
                    let c_cell = region.assign_advice(
                        ||"c",
                        self.config.col_c,
                        0,
                        || c_val.ok.or(Error::Synthesis),
                    ).map(ACell)?;

                    Ok(c_cell)
                },
            )
        }
    }

#[derive(Default)]
struct MyCircuit<F> {
    pub a: Option<F>,
    pub b: Option<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        FiboChip::configure(meta, [col_a, col_b, col_c])
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
       let chip = FiboChip::construct(config); 

       let (_, mut prev_b, mut prev_c) = chip.assign_first_row(
        layouter.namespace(|| "first_row"),
        self.a, self.b,
       );

       for _i in 3..10{
        let (a,b,c) = chip.assign_row(
            layouter.namespace(||"next row"),
            &prev_b,
            &prev_c,
        )?;
        prev_b = prev_c;
        prev_c = c_cell;
       }
       Ok(())
    }
}

fn main() {
    let k = 4;
    let a = Fp::from(1);
    let b = Fp::from(1);
    
    let circuit = MyCircuit{
        a: Some(a),
        b: Some(b),
    };

    let prover: MockProver::run(k, &circuit, vec![]).unwrap();
    prover.assert_satisfied();

    println!("Hello, world!");
}
