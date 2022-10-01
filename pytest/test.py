import pytest

import pyrevm

def test_revm():
    evm = pyrevm.EVM();
    print(evm.foo())
    assert evm.foo() == 1
