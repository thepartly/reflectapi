# Testing Your API

Now that you have a complete Pet Store API, let's test it thoroughly using generated clients and various testing strategies. ReflectAPI makes testing easy by generating clients in multiple languages.

## What We'll Cover

1. **Manual testing** with curl and browser tools
2. **Generated client testing** in TypeScript, Rust, and Python
3. **Automated testing** with comprehensive test suites
4. **Load testing** for performance validation
5. **Error scenario testing** for robustness

## Manual Testing with curl

Start your server:

```bash
cargo run
```

### Basic Functionality Tests

```bash
# Health check
curl http://localhost:3000/health.check

# Create a pet
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{
    "name": "Buddy",
    "kind": {
      "type": "dog",
      "breed": "Golden Retriever"
    },
    "age": 3,
    "behaviors": [
      "Calm",
      {
        "Playful": {
          "favorite_toy": "Tennis Ball"
        }
      }
    ]
  }'

# List pets
curl "http://localhost:3000/pets.list?limit=10" \
  -H "Authorization: Bearer demo-api-key"

# Get a specific pet (use ID from create response)
curl -X POST http://localhost:3000/pets.get \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"id": 1}'

# Update a pet
curl -X POST http://localhost:3000/pets.update \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{
    "id": 1,
    "name": "Buddy the Great",
    "age": 4
  }'

# Delete a pet
curl -X POST http://localhost:3000/pets.delete \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"id": 1}'
```

### Error Scenario Tests

```bash
# Test authentication failure
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -d '{"name": "No Auth", "kind": {"type": "dog", "breed": "Test"}}'

# Test validation errors
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{
    "name": "",
    "kind": {"type": "cat", "lives": 0},
    "age": 200
  }'

# Test name conflict
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "Duplicate", "kind": {"type": "dog", "breed": "Test"}}'

curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "Duplicate", "kind": {"type": "cat", "lives": 5}}'

# Test not found
curl -X POST http://localhost:3000/pets.get \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"id": 99999}'
```

## Generating Clients

First, generate the ReflectAPI schema:

```bash
# Generate schema file
cargo run --bin reflectapi-cli -- schema \
  --input-type build \
  --output pet-store-schema.json \
  --build-fn pet_store_api::api::create_api

# or if using a different module structure:
cargo run --bin reflectapi-cli -- schema \
  --input-type build \
  --output pet-store-schema.json \
  --build-fn crate::api::create_api
```

### Generate TypeScript Client

```bash
# Generate TypeScript client
cargo run --bin reflectapi-cli -- codegen \
  --language typescript \
  --schema pet-store-schema.json \
  --output clients/typescript/

# Install dependencies
cd clients/typescript
npm init -y
npm install axios @types/node typescript ts-node
npm install --save-dev @types/jest jest
```

Create a TypeScript test file `clients/typescript/test.ts`:

```typescript
import { createClient } from './index';

interface CreatePetRequest {
  name: string;
  kind: PetKind;
  age?: number;
  behaviors?: Behavior[];
}

interface PetKind {
  type: "dog" | "cat" | "bird";
  breed?: string;
  lives?: number;
  can_talk?: boolean;
  wingspan_cm?: number;
}

type Behavior = 
  | "Calm"
  | { Aggressive: { level: number; notes: string } }
  | { Playful: { favorite_toy?: string } }
  | { Other: { description: string } };

async function testPetStoreAPI() {
  const client = createClient({
    baseURL: 'http://localhost:3000',
    headers: {
      'Authorization': 'Bearer demo-api-key'
    }
  });

  try {
    console.log('🏥 Testing health check...');
    const health = await client.health.check();
    console.log('✅ Health check passed:', health);

    console.log('\n🐕 Creating a dog...');
    const dog = await client.pets.create({
      name: "Rex",
      kind: {
        type: "dog",
        breed: "German Shepherd"
      },
      age: 5,
      behaviors: [
        "Calm",
        {
          Playful: {
            favorite_toy: "Frisbee"
          }
        }
      ]
    });
    console.log('✅ Dog created:', dog);

    console.log('\n🐱 Creating a cat...');
    const cat = await client.pets.create({
      name: "Whiskers", 
      kind: {
        type: "cat",
        lives: 9
      },
      age: 2,
      behaviors: [
        {
          Other: {
            description: "Loves to sleep in sunbeams"
          }
        }
      ]
    });
    console.log('✅ Cat created:', cat);

    console.log('\n🐦 Creating a bird...');
    const bird = await client.pets.create({
      name: "Polly",
      kind: {
        type: "bird",
        can_talk: true,
        wingspan_cm: 50
      },
      behaviors: ["Calm"]
    });
    console.log('✅ Bird created:', bird);

    console.log('\n📋 Listing all pets...');
    const allPets = await client.pets.list({
      limit: 10
    });
    console.log('✅ Pet list:', allPets);

    console.log('\n🐕 Filtering dogs...');
    const dogs = await client.pets.list({
      kind_filter: "dog",
      limit: 5
    });
    console.log('✅ Dogs found:', dogs);

    console.log('\n🔍 Getting specific pet...');
    const petDetails = await client.pets.get({
      id: dog.pet.id
    });
    console.log('✅ Pet details:', petDetails);

    console.log('\n✏️ Updating pet...');
    const updatedPet = await client.pets.update({
      id: dog.pet.id,
      name: "Rex the Magnificent",
      age: 6
    });
    console.log('✅ Pet updated:', updatedPet);

    console.log('\n🗑️ Deleting cat...');
    const deletedCat = await client.pets.delete({
      id: cat.pet.id
    });
    console.log('✅ Cat deleted:', deletedCat);

    console.log('\n📋 Final pet list...');
    const finalList = await client.pets.list({});
    console.log('✅ Final pets:', finalList);

  } catch (error) {
    console.error('❌ Test failed:', error);
    
    if (error.response) {
      console.error('Response status:', error.response.status);
      console.error('Response data:', error.response.data);
    }
  }
}

// Error handling tests
async function testErrorScenarios() {
  const client = createClient({
    baseURL: 'http://localhost:3000',
    headers: {
      'Authorization': 'Bearer demo-api-key'
    }
  });

  console.log('\n🚨 Testing error scenarios...');

  // Test validation errors
  try {
    await client.pets.create({
      name: "", // Empty name should fail
      kind: { type: "dog", breed: "Test" }
    });
  } catch (error) {
    console.log('✅ Validation error caught:', error.response?.data);
  }

  // Test not found error
  try {
    await client.pets.get({ id: 99999 });
  } catch (error) {
    console.log('✅ Not found error caught:', error.response?.data);
  }

  // Test name conflict
  try {
    await client.pets.create({
      name: "Conflict Test",
      kind: { type: "dog", breed: "Test" }
    });
    
    // Try to create another with same name
    await client.pets.create({
      name: "Conflict Test",
      kind: { type: "cat", lives: 5 }
    });
  } catch (error) {
    console.log('✅ Conflict error caught:', error.response?.data);
  }
}

async function runAllTests() {
  await testPetStoreAPI();
  await testErrorScenarios();
  console.log('\n🎉 All tests completed!');
}

runAllTests().catch(console.error);
```

Run the TypeScript tests:

```bash
npx ts-node test.ts
```

### Generate Rust Client

```bash
# Generate Rust client
cargo run --bin reflectapi-cli -- codegen \
  --language rust \
  --schema pet-store-schema.json \
  --output clients/rust/

# Create a test crate
cd clients/rust
cargo init --name pet-store-client
cd ..
```

Create a Rust test file `clients/rust/src/main.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Include the generated client code
include!("generated.rs");

#[derive(Debug, Serialize)]
struct CreatePetRequest {
    name: String,
    kind: PetKind,
    age: Option<u8>,
    behaviors: Vec<Behavior>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let base_url = "http://localhost:3000";
    let auth_header = "Bearer demo-api-key";

    println!("🏥 Testing health check...");
    let health_response = client
        .post(&format!("{}/health.check", base_url))
        .send()
        .await?;
    println!("✅ Health check: {}", health_response.status());

    println!("\n🐕 Creating a dog...");
    let create_dog_request = CreatePetRequest {
        name: "Rust Dog".to_string(),
        kind: PetKind::Dog {
            breed: "Rust Pointer".to_string(),
        },
        age: Some(3),
        behaviors: vec![
            Behavior::Calm,
            Behavior::Playful {
                favorite_toy: Some("Bone".to_string()),
            },
        ],
    };

    let create_response = client
        .post(&format!("{}/pets.create", base_url))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&create_dog_request)
        .send()
        .await?;

    if create_response.status().is_success() {
        let created_pet: CreatePetResponse = create_response.json().await?;
        println!("✅ Dog created: {:?}", created_pet);

        println!("\n📋 Listing pets...");
        let list_request = serde_json::json!({
            "limit": 10
        });

        let list_response = client
            .post(&format!("{}/pets.list", base_url))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&list_request)
            .send()
            .await?;

        if list_response.status().is_success() {
            let pets: PaginatedPets = list_response.json().await?;
            println!("✅ Pet list: {:?}", pets);
        }

        println!("\n🔍 Getting specific pet...");
        let get_request = serde_json::json!({
            "id": created_pet.pet.id
        });

        let get_response = client
            .post(&format!("{}/pets.get", base_url))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&get_request)
            .send()
            .await?;

        if get_response.status().is_success() {
            let pet: Pet = get_response.json().await?;
            println!("✅ Pet details: {:?}", pet);
        }

        println!("\n✏️ Updating pet...");
        let update_request = serde_json::json!({
            "id": created_pet.pet.id,
            "name": "Updated Rust Dog",
            "age": 4
        });

        let update_response = client
            .post(&format!("{}/pets.update", base_url))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&update_request)
            .send()
            .await?;

        if update_response.status().is_success() {
            let updated_pet: Pet = update_response.json().await?;
            println!("✅ Pet updated: {:?}", updated_pet);
        }

        println!("\n🗑️ Deleting pet...");
        let delete_request = serde_json::json!({
            "id": created_pet.pet.id
        });

        let delete_response = client
            .post(&format!("{}/pets.delete", base_url))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&delete_request)
            .send()
            .await?;

        if delete_response.status().is_success() {
            let deleted_response: DeletePetResponse = delete_response.json().await?;
            println!("✅ Pet deleted: {:?}", deleted_response);
        }

    } else {
        println!("❌ Failed to create dog: {}", create_response.status());
        let error_text = create_response.text().await?;
        println!("Error details: {}", error_text);
    }

    println!("\n🚨 Testing error scenarios...");
    test_error_scenarios(&client, base_url, auth_header).await?;

    println!("\n🎉 All Rust tests completed!");
    Ok(())
}

async fn test_error_scenarios(
    client: &Client,
    base_url: &str,
    auth_header: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Test validation error
    println!("Testing validation error...");
    let invalid_request = serde_json::json!({
        "name": "",
        "kind": {"type": "dog", "breed": "Test"}
    });

    let response = client
        .post(&format!("{}/pets.create", base_url))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&invalid_request)
        .send()
        .await?;

    if !response.status().is_success() {
        println!("✅ Validation error caught: {}", response.status());
        let error_text = response.text().await?;
        println!("Error details: {}", error_text);
    }

    // Test authentication error
    println!("Testing authentication error...");
    let response = client
        .post(&format!("{}/pets.create", base_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "name": "Test",
            "kind": {"type": "dog", "breed": "Test"}
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        println!("✅ Auth error caught: {}", response.status());
    }

    // Test not found error
    println!("Testing not found error...");
    let response = client
        .post(&format!("{}/pets.get", base_url))
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({"id": 99999}))
        .send()
        .await?;

    if !response.status().is_success() {
        println!("✅ Not found error caught: {}", response.status());
    }

    Ok(())
}
```

Update `clients/rust/Cargo.toml`:

```toml
[package]
name = "pet-store-client"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

Run the Rust tests:

```bash
cd clients/rust
cargo run
```

### Generate Python Client

```bash
# Generate Python client  
cargo run --bin reflectapi-cli -- codegen \
  --language python \
  --schema pet-store-schema.json \
  --output clients/python/

cd clients/python
```

Create a Python test file `clients/python/test_client.py`:

```python
import asyncio
import httpx
from typing import Optional, List, Union, Any
import json

# Import the generated client code
from generated import *

async def test_pet_store_api():
    """Test the Pet Store API with comprehensive scenarios."""
    
    async with httpx.AsyncClient() as client:
        base_url = "http://localhost:3000"
        headers = {"Authorization": "Bearer demo-api-key"}
        
        print("🏥 Testing health check...")
        response = await client.post(f"{base_url}/health.check")
        print(f"✅ Health check: {response.status_code}")
        if response.status_code == 200:
            print(f"Health data: {response.json()}")
        
        print("\n🐕 Creating a dog...")
        dog_data = {
            "name": "Python Pup",
            "kind": {
                "type": "dog",
                "breed": "Python Retriever"
            },
            "age": 2,
            "behaviors": [
                "Calm",
                {
                    "Playful": {
                        "favorite_toy": "Python Snake Toy"
                    }
                }
            ]
        }
        
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json=dog_data
        )
        
        if response.status_code == 200:
            created_dog = response.json()
            print(f"✅ Dog created: {created_dog}")
            dog_id = created_dog["pet"]["id"]
            
            print("\n🐱 Creating a cat...")
            cat_data = {
                "name": "Python Cat",
                "kind": {
                    "type": "cat", 
                    "lives": 7
                },
                "age": 3,
                "behaviors": [
                    {
                        "Other": {
                            "description": "Likes to debug Python code"
                        }
                    }
                ]
            }
            
            response = await client.post(
                f"{base_url}/pets.create",
                headers=headers,
                json=cat_data
            )
            
            if response.status_code == 200:
                created_cat = response.json()
                print(f"✅ Cat created: {created_cat}")
                cat_id = created_cat["pet"]["id"]
                
                print("\n🐦 Creating a bird...")
                bird_data = {
                    "name": "Python Parrot",
                    "kind": {
                        "type": "bird",
                        "can_talk": True,
                        "wingspan_cm": 75
                    },
                    "behaviors": ["Calm"]
                }
                
                response = await client.post(
                    f"{base_url}/pets.create",
                    headers=headers,
                    json=bird_data
                )
                
                if response.status_code == 200:
                    created_bird = response.json()
                    print(f"✅ Bird created: {created_bird}")
                    
                    print("\n📋 Listing all pets...")
                    response = await client.post(
                        f"{base_url}/pets.list",
                        headers=headers,
                        json={"limit": 10}
                    )
                    
                    if response.status_code == 200:
                        all_pets = response.json()
                        print(f"✅ All pets: {all_pets}")
                        
                        print("\n🐕 Filtering dogs...")
                        response = await client.post(
                            f"{base_url}/pets.list",
                            headers=headers,
                            json={"kind_filter": "dog", "limit": 5}
                        )
                        
                        if response.status_code == 200:
                            dogs = response.json()
                            print(f"✅ Dogs found: {dogs}")
                        
                        print("\n🔍 Getting specific pet...")
                        response = await client.post(
                            f"{base_url}/pets.get",
                            headers=headers,
                            json={"id": dog_id}
                        )
                        
                        if response.status_code == 200:
                            pet_details = response.json()
                            print(f"✅ Pet details: {pet_details}")
                        
                        print("\n✏️ Updating pet...")
                        response = await client.post(
                            f"{base_url}/pets.update",
                            headers=headers,
                            json={
                                "id": dog_id,
                                "name": "Updated Python Pup",
                                "age": 3
                            }
                        )
                        
                        if response.status_code == 200:
                            updated_pet = response.json()
                            print(f"✅ Pet updated: {updated_pet}")
                        
                        print("\n🗑️ Deleting cat...")
                        response = await client.post(
                            f"{base_url}/pets.delete",
                            headers=headers,
                            json={"id": cat_id}
                        )
                        
                        if response.status_code == 200:
                            deleted_cat = response.json()
                            print(f"✅ Cat deleted: {deleted_cat}")
                        
                        print("\n📋 Final pet list...")
                        response = await client.post(
                            f"{base_url}/pets.list",
                            headers=headers,
                            json={}
                        )
                        
                        if response.status_code == 200:
                            final_pets = response.json()
                            print(f"✅ Final pets: {final_pets}")
        else:
            print(f"❌ Failed to create dog: {response.status_code}")
            print(f"Error: {response.text}")

async def test_error_scenarios():
    """Test various error scenarios."""
    
    async with httpx.AsyncClient() as client:
        base_url = "http://localhost:3000"
        headers = {"Authorization": "Bearer demo-api-key"}
        
        print("\n🚨 Testing error scenarios...")
        
        # Test validation errors
        print("Testing validation errors...")
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={
                "name": "",  # Empty name should fail
                "kind": {"type": "cat", "lives": 0},  # 0 lives should fail
                "age": 200  # Age too high should fail
            }
        )
        
        if response.status_code != 200:
            print(f"✅ Validation error caught: {response.status_code}")
            print(f"Error details: {response.json()}")
        
        # Test authentication error
        print("\nTesting authentication error...")
        response = await client.post(
            f"{base_url}/pets.create",
            json={"name": "No Auth", "kind": {"type": "dog", "breed": "Test"}}
        )
        
        if response.status_code != 200:
            print(f"✅ Auth error caught: {response.status_code}")
        
        # Test not found error
        print("\nTesting not found error...")
        response = await client.post(
            f"{base_url}/pets.get",
            headers=headers,
            json={"id": 99999}
        )
        
        if response.status_code != 200:
            print(f"✅ Not found error caught: {response.status_code}")
            print(f"Error details: {response.json()}")
        
        # Test name conflict
        print("\nTesting name conflict...")
        # Create first pet
        await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={"name": "Conflict Pet", "kind": {"type": "dog", "breed": "Test"}}
        )
        
        # Try to create second pet with same name
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={"name": "Conflict Pet", "kind": {"type": "cat", "lives": 5}}
        )
        
        if response.status_code != 200:
            print(f"✅ Conflict error caught: {response.status_code}")
            print(f"Error details: {response.json()}")

async def main():
    """Run all tests."""
    await test_pet_store_api()
    await test_error_scenarios()
    print("\n🎉 All Python tests completed!")

if __name__ == "__main__":
    asyncio.run(main())
```

Install Python dependencies and run tests:

```bash
pip install httpx
python test_client.py
```

## Automated Testing with Pytest

Create a comprehensive test suite `clients/python/test_comprehensive.py`:

```python
import pytest
import httpx
import asyncio
from typing import Dict, Any

@pytest.fixture
async def client():
    async with httpx.AsyncClient() as client:
        yield client

@pytest.fixture
def headers():
    return {"Authorization": "Bearer demo-api-key"}

@pytest.fixture
def base_url():
    return "http://localhost:3000"

class TestPetStoreAPI:
    
    @pytest.mark.asyncio
    async def test_health_check(self, client, base_url):
        """Test the health check endpoint."""
        response = await client.post(f"{base_url}/health.check")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "healthy"
        assert "timestamp" in data
    
    @pytest.mark.asyncio
    async def test_create_dog(self, client, base_url, headers):
        """Test creating a dog."""
        dog_data = {
            "name": "Test Dog",
            "kind": {"type": "dog", "breed": "Test Breed"},
            "age": 5,
            "behaviors": ["Calm"]
        }
        
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json=dog_data
        )
        
        assert response.status_code == 200
        data = response.json()
        assert data["pet"]["name"] == "Test Dog"
        assert data["pet"]["kind"]["type"] == "dog"
        assert data["pet"]["age"] == 5
        return data["pet"]["id"]
    
    @pytest.mark.asyncio
    async def test_create_cat(self, client, base_url, headers):
        """Test creating a cat."""
        cat_data = {
            "name": "Test Cat",
            "kind": {"type": "cat", "lives": 9},
            "age": 3,
            "behaviors": ["Calm"]
        }
        
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json=cat_data
        )
        
        assert response.status_code == 200
        data = response.json()
        assert data["pet"]["name"] == "Test Cat"
        assert data["pet"]["kind"]["lives"] == 9
    
    @pytest.mark.asyncio
    async def test_create_bird(self, client, base_url, headers):
        """Test creating a bird."""
        bird_data = {
            "name": "Test Bird",
            "kind": {"type": "bird", "can_talk": True, "wingspan_cm": 50},
            "behaviors": ["Calm"]
        }
        
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json=bird_data
        )
        
        assert response.status_code == 200
        data = response.json()
        assert data["pet"]["name"] == "Test Bird"
        assert data["pet"]["kind"]["can_talk"] is True
    
    @pytest.mark.asyncio
    async def test_list_pets(self, client, base_url, headers):
        """Test listing pets."""
        # Create a pet first
        await self.test_create_dog(client, base_url, headers)
        
        response = await client.post(
            f"{base_url}/pets.list",
            headers=headers,
            json={"limit": 10}
        )
        
        assert response.status_code == 200
        data = response.json()
        assert "pets" in data
        assert "total_count" in data
        assert len(data["pets"]) >= 1
    
    @pytest.mark.asyncio
    async def test_get_pet(self, client, base_url, headers):
        """Test getting a specific pet."""
        # Create a pet first
        pet_id = await self.test_create_dog(client, base_url, headers)
        
        response = await client.post(
            f"{base_url}/pets.get",
            headers=headers,
            json={"id": pet_id}
        )
        
        assert response.status_code == 200
        data = response.json()
        assert data["id"] == pet_id
        assert data["name"] == "Test Dog"
    
    @pytest.mark.asyncio
    async def test_update_pet(self, client, base_url, headers):
        """Test updating a pet."""
        # Create a pet first
        pet_id = await self.test_create_dog(client, base_url, headers)
        
        response = await client.post(
            f"{base_url}/pets.update",
            headers=headers,
            json={
                "id": pet_id,
                "name": "Updated Dog",
                "age": 6
            }
        )
        
        assert response.status_code == 200
        data = response.json()
        assert data["name"] == "Updated Dog"
        assert data["age"] == 6
    
    @pytest.mark.asyncio
    async def test_delete_pet(self, client, base_url, headers):
        """Test deleting a pet."""
        # Create a pet first
        pet_id = await self.test_create_dog(client, base_url, headers)
        
        response = await client.post(
            f"{base_url}/pets.delete",
            headers=headers,
            json={"id": pet_id}
        )
        
        assert response.status_code == 200
        data = response.json()
        assert "message" in data
        assert data["deleted_pet"]["id"] == pet_id

class TestErrorScenarios:
    
    @pytest.mark.asyncio
    async def test_missing_auth(self, client, base_url):
        """Test missing authentication."""
        response = await client.post(
            f"{base_url}/pets.create",
            json={"name": "Test", "kind": {"type": "dog", "breed": "Test"}}
        )
        assert response.status_code == 401
    
    @pytest.mark.asyncio
    async def test_invalid_auth(self, client, base_url):
        """Test invalid authentication."""
        response = await client.post(
            f"{base_url}/pets.create",
            headers={"Authorization": "Bearer invalid-key"},
            json={"name": "Test", "kind": {"type": "dog", "breed": "Test"}}
        )
        assert response.status_code == 401
    
    @pytest.mark.asyncio
    async def test_validation_errors(self, client, base_url, headers):
        """Test validation errors."""
        # Empty name
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={"name": "", "kind": {"type": "dog", "breed": "Test"}}
        )
        assert response.status_code == 400
        
        # Invalid cat lives
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={"name": "Test Cat", "kind": {"type": "cat", "lives": 0}}
        )
        assert response.status_code == 400
        
        # Age too high
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={
                "name": "Old Pet",
                "kind": {"type": "dog", "breed": "Test"},
                "age": 200
            }
        )
        assert response.status_code == 400
    
    @pytest.mark.asyncio
    async def test_not_found(self, client, base_url, headers):
        """Test not found errors."""
        response = await client.post(
            f"{base_url}/pets.get",
            headers=headers,
            json={"id": 99999}
        )
        assert response.status_code == 404
    
    @pytest.mark.asyncio
    async def test_name_conflict(self, client, base_url, headers):
        """Test name conflict errors."""
        # Create first pet
        await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={"name": "Duplicate Name", "kind": {"type": "dog", "breed": "Test"}}
        )
        
        # Try to create another with same name
        response = await client.post(
            f"{base_url}/pets.create",
            headers=headers,
            json={"name": "Duplicate Name", "kind": {"type": "cat", "lives": 5}}
        )
        assert response.status_code == 409

if __name__ == "__main__":
    pytest.main([__file__, "-v"])
```

Run comprehensive tests:

```bash
pip install pytest pytest-asyncio
pytest test_comprehensive.py -v
```

## Load Testing

Create a simple load test `load_test.py`:

```python
import asyncio
import httpx
import time
from concurrent.futures import ThreadPoolExecutor

async def create_pet(client, pet_number: int):
    """Create a single pet."""
    try:
        response = await client.post(
            "http://localhost:3000/pets.create",
            headers={"Authorization": "Bearer demo-api-key"},
            json={
                "name": f"Load Test Pet {pet_number}",
                "kind": {"type": "dog", "breed": "Load Tester"},
                "age": pet_number % 20 + 1,  # Age between 1-20
                "behaviors": ["Calm"]
            }
        )
        return response.status_code == 200
    except Exception as e:
        print(f"Error creating pet {pet_number}: {e}")
        return False

async def load_test(concurrent_requests: int, total_requests: int):
    """Run a load test with specified parameters."""
    print(f"🚀 Starting load test: {total_requests} requests with {concurrent_requests} concurrent")
    
    start_time = time.time()
    successful_requests = 0
    failed_requests = 0
    
    async with httpx.AsyncClient(timeout=30.0) as client:
        semaphore = asyncio.Semaphore(concurrent_requests)
        
        async def rate_limited_create_pet(pet_number):
            async with semaphore:
                success = await create_pet(client, pet_number)
                return success
        
        # Create tasks for all requests
        tasks = [
            rate_limited_create_pet(i) 
            for i in range(total_requests)
        ]
        
        # Execute all tasks
        results = await asyncio.gather(*tasks, return_exceptions=True)
        
        # Count results
        for result in results:
            if isinstance(result, Exception):
                failed_requests += 1
            elif result:
                successful_requests += 1
            else:
                failed_requests += 1
    
    end_time = time.time()
    duration = end_time - start_time
    requests_per_second = total_requests / duration
    
    print(f"📊 Load Test Results:")
    print(f"   Total requests: {total_requests}")
    print(f"   Successful: {successful_requests}")
    print(f"   Failed: {failed_requests}")
    print(f"   Duration: {duration:.2f} seconds")
    print(f"   Requests per second: {requests_per_second:.2f}")
    print(f"   Success rate: {(successful_requests/total_requests)*100:.1f}%")

async def main():
    """Run different load test scenarios."""
    
    # Light load test
    print("🧪 Light load test (10 concurrent, 50 total)")
    await load_test(10, 50)
    
    # Medium load test
    print("\n🧪 Medium load test (25 concurrent, 100 total)")
    await load_test(25, 100)
    
    # Heavy load test
    print("\n🧪 Heavy load test (50 concurrent, 200 total)")
    await load_test(50, 200)

if __name__ == "__main__":
    asyncio.run(main())
```

Run load tests:

```bash
python load_test.py
```

## Performance Monitoring

Create a performance monitoring script `performance_monitor.py`:

```python
import asyncio
import httpx
import time
import statistics
from typing import List

async def measure_endpoint_performance(endpoint: str, data: dict, headers: dict):
    """Measure performance of a single endpoint."""
    times = []
    
    async with httpx.AsyncClient() as client:
        for _ in range(10):  # 10 measurements
            start_time = time.time()
            
            try:
                response = await client.post(
                    f"http://localhost:3000/{endpoint}",
                    headers=headers,
                    json=data
                )
                
                end_time = time.time()
                
                if response.status_code in [200, 201]:
                    times.append((end_time - start_time) * 1000)  # Convert to ms
                else:
                    print(f"❌ Error {response.status_code} for {endpoint}")
                    
            except Exception as e:
                print(f"❌ Exception for {endpoint}: {e}")
    
    if times:
        return {
            'endpoint': endpoint,
            'avg_ms': statistics.mean(times),
            'min_ms': min(times),
            'max_ms': max(times),
            'median_ms': statistics.median(times),
            'std_dev': statistics.stdev(times) if len(times) > 1 else 0
        }
    else:
        return None

async def performance_test():
    """Run comprehensive performance tests."""
    headers = {"Authorization": "Bearer demo-api-key"}
    
    test_cases = [
        {
            'endpoint': 'health.check',
            'data': {}
        },
        {
            'endpoint': 'pets.create',
            'data': {
                "name": "Perf Test Pet",
                "kind": {"type": "dog", "breed": "Speed Demon"},
                "age": 3,
                "behaviors": ["Calm"]
            }
        },
        {
            'endpoint': 'pets.list',
            'data': {"limit": 10}
        }
    ]
    
    print("📈 Performance Testing Results")
    print("=" * 50)
    
    for test_case in test_cases:
        result = await measure_endpoint_performance(
            test_case['endpoint'],
            test_case['data'],
            headers
        )
        
        if result:
            print(f"\n🎯 {result['endpoint']}")
            print(f"   Average: {result['avg_ms']:.2f}ms")
            print(f"   Median:  {result['median_ms']:.2f}ms")
            print(f"   Min:     {result['min_ms']:.2f}ms")
            print(f"   Max:     {result['max_ms']:.2f}ms")
            print(f"   Std Dev: {result['std_dev']:.2f}ms")

if __name__ == "__main__":
    asyncio.run(performance_test())
```

## What You've Accomplished

✅ **Manual testing** with comprehensive curl commands  
✅ **Generated client testing** in TypeScript, Rust, and Python  
✅ **Automated test suites** with pytest  
✅ **Load testing** for performance validation  
✅ **Error scenario testing** for robustness  
✅ **Performance monitoring** and metrics  
✅ **Cross-language client validation**  

## Key Testing Benefits of ReflectAPI

1. **Type Safety**: Generated clients catch errors at compile/development time
2. **Consistency**: Same API behavior across all client languages
3. **Documentation**: Types serve as living documentation
4. **Error Handling**: Structured error responses with proper HTTP status codes
5. **Validation**: Request/response validation happens automatically
6. **Performance**: Efficient serialization and minimal overhead

## Production Deployment Checklist

Before deploying your Pet Store API to production:

- [ ] Add proper database instead of in-memory storage
- [ ] Implement proper authentication (JWT, OAuth2, etc.)
- [ ] Add rate limiting middleware
- [ ] Set up monitoring and alerting
- [ ] Configure HTTPS/TLS
- [ ] Add request/response logging
- [ ] Implement health checks and metrics endpoints
- [ ] Set up CI/CD pipeline with automated testing
- [ ] Add caching layers (Redis, etc.)
- [ ] Configure backup and disaster recovery

## Congratulations! 🎉

You've successfully built a complete Pet Store API using ReflectAPI with:

- **Type-safe API definitions** with automatic client generation
- **Comprehensive validation** at multiple levels
- **Production-ready error handling** with structured responses
- **Cross-language client support** (TypeScript, Rust, Python)
- **Thorough testing** including unit, integration, and load tests

Your API is now ready for production deployment and can serve as a foundation for building larger, more complex APIs with ReflectAPI!
