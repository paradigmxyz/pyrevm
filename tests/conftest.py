import time
import pytest

@pytest.fixture(autouse=True)
def slow_down_tests():
    yield
    # Sleep after each test to avoid Infura rate limiting
    time.sleep(0.5)
