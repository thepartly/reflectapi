"""
Flatten support for ReflectAPI Python clients.

This module provides runtime support for serde's #[serde(flatten)] feature,
enabling proper serialization and deserialization of flattened fields in Pydantic models.
"""

from typing import Any, Dict, List, Optional, Type, get_args, get_origin
from pydantic import BaseModel, Field, field_validator, model_validator, model_serializer


class FlattenedModel(BaseModel):
    """
    Base class for models with flattened fields.
    
    This class provides the necessary validators and serializers to handle
    serde's flatten semantics in Python/Pydantic.
    """
    
    # Use class methods to access metadata to avoid Pydantic private field issues
    @classmethod
    def _get_flattened_fields(cls) -> Dict[str, Type[BaseModel]]:
        """Get the flattened field definitions."""
        return getattr(cls, '__flattened_fields__', {})
    
    @classmethod
    def _get_optional_flattened(cls) -> List[str]:
        """Get the list of optional flattened field names."""
        return getattr(cls, '__optional_flattened__', [])
    
    @model_validator(mode='before')
    @classmethod
    def _unflatten_fields(cls, data: Any) -> Any:
        """
        Pre-validation hook that distributes flat input data to nested structures.
        
        This validator runs before Pydantic's field validation and restructures
        the input data to match the actual model structure.
        """
        if not isinstance(data, dict):
            return data
            
        # Make a copy to avoid modifying the original
        data = dict(data)
        
        # Process each flattened field
        for field_name, field_type in cls._get_flattened_fields().items():
            # Skip if the field is already present (already nested)
            if field_name in data:
                continue
                
            # Collect fields that belong to this flattened type
            nested_data = {}
            fields_to_remove = []
            
            # Get the field names from the target model
            if hasattr(field_type, 'model_fields'):
                for nested_field_name in field_type.model_fields:
                    if nested_field_name in data:
                        nested_data[nested_field_name] = data[nested_field_name]
                        fields_to_remove.append(nested_field_name)
            
            # Only create the nested object if we found any of its fields
            if nested_data:
                # Remove the fields from the flat data
                for field in fields_to_remove:
                    del data[field]
                    
                # Add the nested structure
                data[field_name] = nested_data
            elif field_name not in cls._get_optional_flattened():
                # For required flattened fields, create an empty dict to trigger validation
                data[field_name] = {}
        
        return data
    
    @model_serializer(mode='wrap')
    def _flatten_serialization(self, serializer, info):
        """
        Serialization hook that flattens nested structures for output.
        
        This serializer runs during model serialization and flattens
        nested structures according to serde's flatten semantics.
        """
        # Get the normal serialization
        data = serializer(self)
        
        # If not in JSON mode, return as-is
        if not info.mode_is_json():
            return data
            
        # Process flattened fields
        result = {}
        for key, value in data.items():
            if key in self._get_flattened_fields():
                # This is a flattened field - merge its contents into the result
                if value is not None:
                    if isinstance(value, dict):
                        result.update(value)
                    elif hasattr(value, 'model_dump'):
                        result.update(value.model_dump(mode='json'))
            else:
                # Regular field - add as-is
                result[key] = value
                
        return result


def create_flattened_model(
    name: str,
    regular_fields: Dict[str, Any],
    flattened_fields: Dict[str, Type[BaseModel]],
    optional_flattened: Optional[List[str]] = None,
    base_class: Type[BaseModel] = FlattenedModel
) -> Type[BaseModel]:
    """
    Factory function to create a model with flattened fields.
    
    Args:
        name: Name of the model class to create
        regular_fields: Dictionary of regular field names to types
        flattened_fields: Dictionary of field names to types that should be flattened
        optional_flattened: List of flattened field names that are optional
        base_class: Base class to inherit from (default: FlattenedModel)
        
    Returns:
        A new model class with proper flatten support
        
    Example:
        ```python
        # Define the nested model
        class Inner(BaseModel):
            field_a: str
            field_b: int
            
        # Create the outer model with flatten
        Outer = create_flattened_model(
            "Outer",
            regular_fields={"field_c": bool},
            flattened_fields={"inner": Inner}
        )
        
        # Usage
        data = {"field_a": "test", "field_b": 42, "field_c": True}
        obj = Outer.model_validate(data)
        print(obj.model_dump())  # Flattened output
        ```
    """
    # Build the complete field definitions
    field_definitions = {}
    
    # Add regular fields
    for field_name, field_type in regular_fields.items():
        field_definitions[field_name] = (field_type, Field())
    
    # Add flattened fields
    optional_flattened = optional_flattened or []
    for field_name, field_type in flattened_fields.items():
        if field_name in optional_flattened:
            # Optional flattened field
            field_definitions[field_name] = (Optional[field_type], Field(default=None))
        else:
            # Required flattened field
            field_definitions[field_name] = (field_type, Field())
    
    # Create the namespace for the new class
    namespace = {
        '__module__': __name__,
        '__annotations__': {},
        '__flattened_fields__': flattened_fields,
        '__optional_flattened__': optional_flattened or [],
    }
    
    # Add field annotations and defaults
    for field_name, (field_type, field_def) in field_definitions.items():
        namespace['__annotations__'][field_name] = field_type
        namespace[field_name] = field_def
    
    # Create the model class
    model_class = type(name, (base_class,), namespace)
    
    return model_class


class FlattenedFieldDescriptor:
    """
    Descriptor for individual flattened fields, providing attribute-style access
    to fields from flattened structures.
    
    This is useful when you want to access flattened fields directly on the parent model.
    """
    
    def __init__(self, source_field: str, nested_field: str):
        self.source_field = source_field
        self.nested_field = nested_field
    
    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        source = getattr(obj, self.source_field, None)
        if source is None:
            return None
        return getattr(source, self.nested_field, None)
    
    def __set__(self, obj, value):
        source = getattr(obj, self.source_field, None)
        if source is None:
            # Create the nested object if it doesn't exist
            flattened_fields = obj._get_flattened_fields()
            source_type = flattened_fields.get(self.source_field)
            if source_type:
                source = source_type()
                setattr(obj, self.source_field, source)
        if source is not None:
            setattr(source, self.nested_field, value)


# Helper function for handling complex flatten scenarios
def flatten_dict(data: Dict[str, Any], flattened_fields: List[str]) -> Dict[str, Any]:
    """
    Utility function to flatten a dictionary according to specified fields.
    
    Args:
        data: Dictionary to flatten
        flattened_fields: List of field names that should be flattened
        
    Returns:
        Flattened dictionary
    """
    result = {}
    for key, value in data.items():
        if key in flattened_fields and isinstance(value, dict):
            # Merge the nested dict into the result
            result.update(value)
        else:
            result[key] = value
    return result


def unflatten_dict(
    data: Dict[str, Any],
    flattened_specs: Dict[str, Type[BaseModel]]
) -> Dict[str, Any]:
    """
    Utility function to unflatten a dictionary according to field specifications.
    
    Args:
        data: Flat dictionary to unflatten
        flattened_specs: Mapping of field names to their model types
        
    Returns:
        Unflattened dictionary with nested structures
    """
    result = dict(data)
    
    for field_name, field_type in flattened_specs.items():
        if field_name in result:
            # Already nested, skip
            continue
            
        # Collect fields for this nested structure
        nested_data = {}
        if hasattr(field_type, 'model_fields'):
            for nested_field in field_type.model_fields:
                if nested_field in result:
                    nested_data[nested_field] = result.pop(nested_field)
        
        # Add the nested structure if we found any fields
        if nested_data:
            result[field_name] = nested_data
            
    return result