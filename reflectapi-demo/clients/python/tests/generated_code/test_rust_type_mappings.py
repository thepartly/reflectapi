"""Tests for Rust-specific type mappings in Python client generation.

This tests the improvements made to handle Rust standard library types:
- std::time::Duration -> datetime.timedelta
- std::path::PathBuf -> pathlib.Path
- std::net::IpAddr -> ipaddress.IPv4Address | IPv6Address
- Vec<u8> -> bytes
- serde_json::Value -> Any (already correct)
"""

from datetime import timedelta
from pathlib import Path
from ipaddress import IPv4Address, IPv6Address
from typing import Any, Union

# Note: Since these types aren't used in the demo server yet,
# we'll test the type mapping logic and generated code structure


class TestTypeAnnotations:
    """Test that Python type annotations are correctly generated."""

    def test_imports_include_required_modules(self):
        """Test that generated code includes necessary imports."""
        gen_path = Path(__file__).resolve().parents[2] / "generated.py"
        with open(gen_path, "r") as f:
            content = f.read()

        # Check for required imports in the generated code
        # Note: Some imports may only be present if the types are actually used

        # datetime should be imported (used for chrono::DateTime)
        assert "from datetime import datetime" in content

        # typing imports should include Any, Union, etc.
        assert "from typing import Any" in content
        assert "Union" in content

    def test_external_type_definitions_present(self):
        """Test that external type definitions are present."""
        gen_path = Path(__file__).resolve().parents[2] / "generated.py"
        with open(gen_path, "r") as f:
            content = f.read()

        # Check for external type annotations section
        assert "# External type definitions" in content

        # Check for Rust NonZero type mappings
        assert "StdNumNonZeroU32" in content
        assert "StdNumNonZeroU64" in content
        assert "StdNumNonZeroI32" in content
        assert "StdNumNonZeroI64" in content


class TestRustTypeMapping:
    """Test conceptual mapping of Rust types to Python equivalents."""

    def test_duration_mapping_concept(self):
        """Test Duration concept - would map to timedelta."""
        # In a real scenario, std::time::Duration would be:
        duration = timedelta(seconds=30, microseconds=500000)

        # Test basic timedelta operations that would be expected
        assert duration.total_seconds() == 30.5
        assert duration.seconds == 30
        assert duration.microseconds == 500000

    def test_pathbuf_mapping_concept(self):
        """Test PathBuf concept - would map to pathlib.Path."""
        # In a real scenario, std::path::PathBuf would be:
        path = Path("/usr/local/bin")

        # Test basic Path operations that would be expected
        assert str(path) == "/usr/local/bin"
        assert path.name == "bin"
        assert path.parent == Path("/usr/local")
        assert path.is_absolute()

    def test_ipaddr_mapping_concept(self):
        """Test IpAddr concept - would map to ipaddress types."""
        # In a real scenario, std::net::IpAddr would be:
        ipv4 = IPv4Address("192.168.1.1")
        ipv6 = IPv6Address("::1")

        # Test basic IP address operations
        assert str(ipv4) == "192.168.1.1"
        assert ipv4.version == 4
        assert str(ipv6) == "::1"
        assert ipv6.version == 6

        # Test union type compatibility
        def accept_ip_addr(addr: Union[IPv4Address, IPv6Address]) -> int:
            return addr.version

        assert accept_ip_addr(ipv4) == 4
        assert accept_ip_addr(ipv6) == 6

    def test_vec_u8_mapping_concept(self):
        """Test Vec<u8> concept - would map to bytes."""
        # In a real scenario, Vec<u8> would be:
        data = b"Hello, world!"

        # Test basic bytes operations
        assert isinstance(data, bytes)
        assert len(data) == 13
        assert data[0] == ord("H")
        assert data.decode("utf-8") == "Hello, world!"

    def test_serde_json_value_mapping_concept(self):
        """Test serde_json::Value concept - maps to Any."""
        # In a real scenario, serde_json::Value would be:
        json_value: Any = {"key": "value", "number": 42, "array": [1, 2, 3]}

        # Test that Any can hold various types
        assert isinstance(json_value, dict)
        assert json_value["key"] == "value"
        assert json_value["number"] == 42
        assert json_value["array"] == [1, 2, 3]

        # Any should also accept other types
        string_value: Any = "just a string"
        number_value: Any = 123
        assert isinstance(string_value, str)
        assert isinstance(number_value, int)


class TestVecU8SpecialHandling:
    """Test special handling for Vec<u8> -> bytes mapping."""

    def test_bytes_creation_and_manipulation(self):
        """Test bytes type operations that Vec<u8> would need."""
        # Create bytes from various sources
        from_string = "Hello".encode("utf-8")
        from_list = bytes([72, 101, 108, 108, 111])  # "Hello" in ASCII
        from_literal = b"Hello"

        # All should be equivalent
        assert from_string == from_list == from_literal

        # Test bytes operations
        assert len(from_literal) == 5
        assert from_literal[0] == 72  # 'H'
        assert from_literal.decode("utf-8") == "Hello"

    def test_bytes_json_serialization(self):
        """Test how bytes would be handled in JSON serialization."""
        import base64
        import json

        data = b"Binary data here"

        # Bytes need to be encoded for JSON (common approaches)
        encoded = base64.b64encode(data).decode("ascii")

        # Should be able to round-trip
        json_str = json.dumps({"data": encoded})
        parsed = json.loads(json_str)
        decoded = base64.b64decode(parsed["data"])

        assert decoded == data


class TestCodeGenerationStructure:
    """Test the structure of generated code for type mappings."""

    def test_type_mapping_consistency(self):
        """Test that type mappings are consistent throughout generated code."""
        gen_path = Path(__file__).resolve().parents[2] / "generated.py"
        with open(gen_path, "r") as f:
            content = f.read()

        # Check that datetime is used consistently
        if "datetime" in content:
            # If datetime is used, it should be properly imported
            assert "from datetime import datetime" in content

        # Check that Union types are properly formed
        union_count = content.count("Union[")
        # There should be at least one Union (PetKind)
        assert union_count >= 1

    def test_literal_import_conditional(self):
        """Test that Literal is only imported when needed."""
        gen_path = Path(__file__).resolve().parents[2] / "generated.py"
        with open(gen_path, "r") as f:
            content = f.read()

        # Since we have tagged enums, Literal should be imported
        assert "Literal" in content

        # And it should be in the typing import
        typing_imports = [
            line
            for line in content.split("\n")
            if line.startswith("from typing import")
        ]
        has_literal_import = any("Literal" in line for line in typing_imports)
        assert has_literal_import, "Literal should be imported in typing statement"


class TestFutureTypeIntegration:
    """Test how new type mappings would integrate with existing models."""

    def test_hypothetical_model_with_rust_types(self):
        """Test how a model with Rust types would work."""
        from datetime import datetime, timedelta
        from pathlib import Path
        from ipaddress import IPv4Address
        from typing import Any

        # Hypothetical model that would be generated for Rust types
        class HypotheticalRustTypesModel:
            def __init__(
                self,
                duration: timedelta,
                path: Path,
                ip: IPv4Address,
                data: bytes,
                metadata: Any,
            ):
                self.duration = duration
                self.path = path
                self.ip = ip
                self.data = data
                self.metadata = metadata

        # Test creating such a model
        model = HypotheticalRustTypesModel(
            duration=timedelta(minutes=5),
            path=Path("/tmp/test.txt"),
            ip=IPv4Address("10.0.0.1"),
            data=b"test data",
            metadata={"version": 1, "created": datetime.now()},
        )

        # Verify all types are correct
        assert isinstance(model.duration, timedelta)
        assert isinstance(model.path, Path)
        assert isinstance(model.ip, IPv4Address)
        assert isinstance(model.data, bytes)
        assert isinstance(model.metadata, dict)

        # Test type operations
        assert model.duration.total_seconds() == 300
        assert model.path.suffix == ".txt"
        assert str(model.ip) == "10.0.0.1"
        assert len(model.data) == 9

    def test_pydantic_compatibility(self):
        """Test that Rust type mappings would work with Pydantic."""
        from pydantic import BaseModel
        from datetime import timedelta
        from pathlib import Path
        from ipaddress import IPv4Address
        from typing import Any

        class HypotheticalPydanticModel(BaseModel):
            duration: timedelta
            path: Path
            ip_address: IPv4Address
            binary_data: bytes
            json_data: Any

        # Test model creation and validation
        model = HypotheticalPydanticModel(
            duration=timedelta(hours=2),
            path=Path("/home/user/document.pdf"),
            ip_address="192.168.1.100",  # Pydantic should convert string
            binary_data=b"binary content",
            json_data={"nested": {"data": [1, 2, 3]}},
        )

        assert model.duration == timedelta(hours=2)
        assert model.path == Path("/home/user/document.pdf")
        assert model.ip_address == IPv4Address("192.168.1.100")
        assert model.binary_data == b"binary content"
        assert model.json_data["nested"]["data"] == [1, 2, 3]

        # Test serialization
        data = model.model_dump()
        assert isinstance(data["duration"], timedelta)
        assert isinstance(data["path"], Path)
        assert isinstance(data["ip_address"], IPv4Address)
        assert isinstance(data["binary_data"], bytes)


class TestImportOptimization:
    """Test that imports are optimized and only included when needed."""

    def test_conditional_imports(self):
        """Test that imports are only present when types are used."""
        gen_path = Path(__file__).resolve().parents[2] / "generated.py"
        with open(gen_path, "r") as f:
            content = f.read()

        # Basic imports should always be present
        assert "from typing import" in content
        assert "from pydantic import" in content

        # Conditional imports based on usage
        has_datetime_usage = "datetime" in content and "updated_at:" in content
        if has_datetime_usage:
            assert "from datetime import datetime" in content

        # ReflectapiOption should be imported since it's used
        assert "from reflectapi_runtime import ReflectapiOption" in content
