"""
Integration tests for rlm-core upgrade compatibility.

Tests verify that:
1. Version detection works correctly
2. Feature detection returns expected values
3. Graceful degradation when features are unavailable
4. Forward compatibility with new consumers

Run with: pytest tests/integration/test_upgrade_compatibility.py -v
"""

import pytest
import sys
from typing import Tuple


# ============================================================================
# Test Fixtures
# ============================================================================


@pytest.fixture
def rlm_core():
    """Import rlm_core, skip if not available."""
    try:
        import rlm_core
        return rlm_core
    except ImportError:
        pytest.skip("rlm_core not installed - build with: maturin develop --features full")


# ============================================================================
# Version Detection Tests
# ============================================================================


class TestVersionDetection:
    """Test version detection functions."""

    def test_version_string(self, rlm_core):
        """Version string should be non-empty and contain dots."""
        version = rlm_core.version()
        assert isinstance(version, str)
        assert len(version) > 0
        assert "." in version

    def test_version_tuple(self, rlm_core):
        """Version tuple should have three non-negative integers."""
        major, minor, patch = rlm_core.version_tuple()
        assert isinstance(major, int)
        assert isinstance(minor, int)
        assert isinstance(patch, int)
        assert major >= 0
        assert minor >= 0
        assert patch >= 0

    def test_version_consistency(self, rlm_core):
        """Version string and tuple should be consistent."""
        version_str = rlm_core.version()
        major, minor, patch = rlm_core.version_tuple()

        # Version string should start with tuple components
        expected_prefix = f"{major}.{minor}.{patch}"
        assert version_str.startswith(expected_prefix), (
            f"Version '{version_str}' should start with '{expected_prefix}'"
        )

    def test_minimum_version(self, rlm_core):
        """Library should be at least version 0.1.0."""
        major, minor, patch = rlm_core.version_tuple()
        version_num = major * 10000 + minor * 100 + patch
        assert version_num >= 100, "Expected at least version 0.1.0"


# ============================================================================
# Feature Detection Tests
# ============================================================================


class TestFeatureDetection:
    """Test feature detection functions."""

    def test_known_features(self, rlm_core):
        """Known features should return bool values."""
        for feature in ["gemini", "adversarial", "python"]:
            result = rlm_core.has_feature(feature)
            assert isinstance(result, bool), f"has_feature('{feature}') should return bool"

    def test_python_feature_always_available(self, rlm_core):
        """Python feature should always be True when calling from Python."""
        assert rlm_core.has_feature("python") is True

    def test_unknown_feature_raises(self, rlm_core):
        """Unknown features should raise ValueError."""
        with pytest.raises(ValueError) as exc_info:
            rlm_core.has_feature("unknown_feature_xyz")
        assert "Unknown feature" in str(exc_info.value)

    def test_available_features_list(self, rlm_core):
        """Available features should return a list of strings."""
        features = rlm_core.available_features()
        assert isinstance(features, list)
        assert all(isinstance(f, str) for f in features)
        assert "python" in features  # Always available

    def test_feature_consistency(self, rlm_core):
        """has_feature and available_features should be consistent."""
        features = rlm_core.available_features()

        for feature in ["gemini", "adversarial", "python"]:
            has_it = rlm_core.has_feature(feature)
            in_list = feature in features
            assert has_it == in_list, (
                f"has_feature('{feature}')={has_it} but "
                f"'{feature}' in available_features()={in_list}"
            )

    def test_adversarial_implies_gemini(self, rlm_core):
        """If adversarial is available, gemini must also be available."""
        if rlm_core.has_feature("adversarial"):
            assert rlm_core.has_feature("gemini"), (
                "adversarial feature requires gemini feature"
            )


# ============================================================================
# Graceful Degradation Tests
# ============================================================================


class TestGracefulDegradation:
    """Test graceful degradation when features are unavailable."""

    def test_core_types_always_available(self, rlm_core):
        """Core types should always be importable."""
        # These should always exist
        assert hasattr(rlm_core, "SessionContext")
        assert hasattr(rlm_core, "Message")
        assert hasattr(rlm_core, "PatternClassifier")
        assert hasattr(rlm_core, "ActivationDecision")

    def test_adversarial_types_conditional(self, rlm_core):
        """Adversarial types should only exist when feature is enabled."""
        has_adversarial = rlm_core.has_feature("adversarial")

        adversarial_types = [
            "AdversarialConfig",
            "ValidationContext",
            "ValidationResult",
            "IssueSeverity",
        ]

        for type_name in adversarial_types:
            exists = hasattr(rlm_core, type_name)
            if has_adversarial:
                assert exists, f"{type_name} should exist when adversarial feature enabled"
            # Note: We don't assert non-existence when disabled because
            # the test might be running against a full-featured build

    def test_provider_enum(self, rlm_core):
        """Provider enum should exist with core providers."""
        assert hasattr(rlm_core, "Provider")

        # Core providers always available
        assert hasattr(rlm_core.Provider, "Anthropic")
        assert hasattr(rlm_core.Provider, "OpenAI")
        assert hasattr(rlm_core.Provider, "OpenRouter")

        # Google provider conditional on gemini feature
        has_google = hasattr(rlm_core.Provider, "Google")
        has_gemini = rlm_core.has_feature("gemini")

        if has_gemini:
            assert has_google, "Google provider should exist when gemini feature enabled"


# ============================================================================
# Forward Compatibility Tests
# ============================================================================


class TestForwardCompatibility:
    """Test forward compatibility with new consumers."""

    def test_version_comparison_helper(self, rlm_core):
        """Helper to compare versions for consumer code."""

        def version_at_least(major: int, minor: int, patch: int = 0) -> bool:
            """Check if rlm_core version is at least the specified version."""
            cur_major, cur_minor, cur_patch = rlm_core.version_tuple()
            current = (cur_major, cur_minor, cur_patch)
            required = (major, minor, patch)
            return current >= required

        # Should work with current version
        cur_major, cur_minor, cur_patch = rlm_core.version_tuple()
        assert version_at_least(cur_major, cur_minor, cur_patch)
        assert version_at_least(0, 0, 0)

        # Should fail for future versions
        assert not version_at_least(999, 0, 0)

    def test_safe_feature_check(self, rlm_core):
        """Consumer code pattern for safe feature checking."""

        def safe_has_feature(name: str) -> bool:
            """Safely check for a feature, returning False if unknown."""
            try:
                return rlm_core.has_feature(name)
            except (ValueError, AttributeError):
                return False

        # Known features
        assert isinstance(safe_has_feature("gemini"), bool)
        assert isinstance(safe_has_feature("adversarial"), bool)

        # Unknown features return False, not raise
        assert safe_has_feature("future_feature") is False

    def test_optional_import_pattern(self, rlm_core):
        """Consumer code pattern for optional imports."""

        def get_validator():
            """Get adversarial validator if available, else None."""
            if not rlm_core.has_feature("adversarial"):
                return None

            try:
                # Would import AdversarialValidator here
                return "validator_available"
            except (ImportError, AttributeError):
                return None

        result = get_validator()
        if rlm_core.has_feature("adversarial"):
            assert result is not None
        else:
            assert result is None


# ============================================================================
# Cross-Version Simulation Tests
# ============================================================================


class TestCrossVersionSimulation:
    """Simulate cross-version scenarios."""

    def test_old_consumer_pattern(self, rlm_core):
        """
        Old consumer (pre-feature-flags) pattern.

        Old consumers might not know about has_feature() and should
        still work with basic functionality.
        """
        # Old consumer just uses core types
        ctx = rlm_core.SessionContext()
        classifier = rlm_core.PatternClassifier()

        # Should work regardless of features
        decision = classifier.should_activate("simple query", ctx)
        assert hasattr(decision, "should_activate")
        assert hasattr(decision, "score")

    def test_new_consumer_old_core_pattern(self, rlm_core):
        """
        New consumer, old core pattern.

        New consumers should check for features before using them.
        """

        def use_adversarial_if_available(code: str) -> dict:
            """New consumer checks features first."""
            result = {"validated": False, "issues": []}

            # Check version for feature support
            major, minor, _ = rlm_core.version_tuple()
            if (major, minor) < (0, 2):
                # Old core, no adversarial support
                return result

            # Check feature flag
            if not rlm_core.has_feature("adversarial"):
                return result

            # Would use AdversarialValidator here
            result["validated"] = True
            return result

        result = use_adversarial_if_available("def foo(): pass")
        assert isinstance(result, dict)
        assert "validated" in result

    def test_gemini_unavailable_fallback(self, rlm_core):
        """Test fallback when Gemini provider is unavailable."""

        def get_cross_provider_client():
            """Get a cross-provider validator, with fallback."""
            if rlm_core.has_feature("gemini"):
                return {"provider": "gemini", "available": True}
            else:
                # Fall back to OpenAI or local validation
                return {"provider": "fallback", "available": True}

        result = get_cross_provider_client()
        assert result["available"] is True
        if rlm_core.has_feature("gemini"):
            assert result["provider"] == "gemini"
        else:
            assert result["provider"] == "fallback"


# ============================================================================
# Main
# ============================================================================


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
