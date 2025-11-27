## Openvm issue while handling big inputs

Use `make` to do test:

~~~bash
make test
~~~

or
~~~bash
make test-failure
~~~

+ `test_execute_cost` load an guest app built by up-to-date openvm (1.4.1)
+ `test_execute_cost_legacy` load an guest app built legacy openvm (1.3), use [some hacking for compability](https://github.com/scroll-tech/zkvm-prover/blob/56c951893bac4754a170dd95fa186d21aa34e2bf/crates/prover/src/setup.rs#L22)

