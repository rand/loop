"""
Pytest configuration for rlm-core tests.
"""

import pytest
import sys
from pathlib import Path


def pytest_configure(config):
    """Configure pytest markers."""
    config.addinivalue_line(
        "markers", "integration: mark test as an integration test"
    )
    config.addinivalue_line(
        "markers", "requires_gemini: mark test as requiring gemini feature"
    )
    config.addinivalue_line(
        "markers", "requires_adversarial: mark test as requiring adversarial feature"
    )


@pytest.fixture(scope="session")
def rlm_core_module():
    """Session-scoped fixture for rlm_core module."""
    try:
        import rlm_core
        return rlm_core
    except ImportError:
        pytest.skip(
            "rlm_core not installed. Build with: "
            "cd rlm-core && maturin develop --features full"
        )


@pytest.fixture
def has_gemini(rlm_core_module):
    """Check if gemini feature is available."""
    return rlm_core_module.has_feature("gemini")


@pytest.fixture
def has_adversarial(rlm_core_module):
    """Check if adversarial feature is available."""
    return rlm_core_module.has_feature("adversarial")
