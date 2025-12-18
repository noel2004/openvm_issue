## Core dump while proving
Use `make` to reproduce the issue:

~~~bash
make test-core-dump
~~~

or
~~~bash
make test-core-dump-cpu
~~~

You can verify the input and app is correct by do an execution instead of proving, which should be success
~~~bash
make test-core-dump-execute
~~~

The issue can be resolved if we also use the `legacy` feature:
~~~bash
make test-core-dump-passed
~~~


## Openvm issue while handling big inputs

Use `make` to do test:

~~~bash
make test-meter
~~~

or
~~~bash
make test-meter-failure
~~~

+ `test_execute_cost` load an guest app built by up-to-date openvm (1.4.1)
+ `test_execute_cost_legacy` load an guest app built legacy openvm (1.3), use [some hacking for compability](https://github.com/scroll-tech/zkvm-prover/blob/56c951893bac4754a170dd95fa186d21aa34e2bf/crates/prover/src/setup.rs#L22)

