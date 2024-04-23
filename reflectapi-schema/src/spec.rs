




// export const EasyApiImplementationTypes: Array<EasyApiImplementationType> = [
// 	{
// 		kind: 'implementation',
// 		name: 'string',
// 		namespace: 'std',
// 		description: 'UTF-8 string',
// 		implementations: [
// 			{
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'String',
// 				to_string: true,
// 				from_string: true
// 			},
// 			{
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'string',
// 				to_string: 'value',
// 				from_string: 'value'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: 'String'
// 			}
// 		]
// 	},
// 	{
// 		kind: 'implementation',
// 		name: 'bool',
// 		namespace: 'std',
// 		description: 'Boolean value',
// 		implementations: [
// 			{
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'bool',
// 				to_string: true,
// 				from_string: true
// 			},
// 			{
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'boolean',
// 				to_string: 'value.toString()',
// 				from_string: '(value === "true")'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: 'Boolean'
// 			}
// 		]
// 	},
// 	{
// 		kind: 'implementation',
// 		name: 'uuid',
// 		namespace: 'std',
// 		description: 'UUID value',
// 		implementations: [
// 			{
// 				// msgpack representation is bytes
// 				language: 'rust',
// 				tags: ['easyapi', 'json:string', 'msgpack:bytes'],
// 				type: 'uuid::Uuid',
// 				to_string: true,
// 				from_string: true
// 			},
// 			// TODO rust and typescript implementations are not compatible for msgpack
// 			{
// 				// msgpack representation is string, needs to be bytes
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'string',
// 				to_string: 'value',
// 				from_string: 'value',
// 				to_msgpack: 'TODO',
// 				from_msgpack: 'TODO'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: 'ID'
// 			}
// 		]
// 	},
// 	{
// 		kind: 'implementation',
// 		name: 'datetime',
// 		namespace: 'std',
// 		description: 'Datetime value',
// 		implementations: [
// 			// TODO this needs multiple implementations
// 			// - lookup-client uses 'new Date(Number(entity.${typescriptColumnName}) / 1000)'
// 			// - data platform client rs 2 uses chrono::DateTime<chrono::offset::Utc>
// 			{
// 				// TODO what is msgpack representation? timestamp?
// 				// json representation is string
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'chrono::DateTime<chrono::offset::Utc>',
// 				to_string: true,
// 				from_string: true
// 			},
// 			{
// 				// TODO this needs one more implementation with better more specific type
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'string',
// 				to_string: 'value',
// 				from_string: 'value'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: 'Date' // in graphql standard this is DateTime type, but partly overrides it with a scalar
// 			}
// 		]
// 	},
// 	{
// 		kind: 'implementation',
// 		name: 'json',
// 		namespace: 'std',
// 		description: 'Any JSON value',
// 		implementations: [
// 			{
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'serde_json::Value'
// 			},
// 			{
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'any'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: 'JSON' // needs https://github.com/taion/graphql-type-json
// 			}
// 		]
// 	},
// 	...['i8', 'i16', 'i32', 'i64', 'i128', 'isize', 'u8', 'u16', 'u32', 'u64', 'usize'].map(
// 		i =>
// 			({
// 				kind: 'implementation',
// 				name: i,
// 				namespace: 'std',
// 				description: `${i.replace(/[iu]/, '')}-bit ${
// 					i.startsWith('u') ? 'unsigned' : 'signed'
// 				} integer`,
// 				implementations: [
// 					{
// 						language: 'rust',
// 						tags: ['easyapi'],
// 						type: i,
// 						to_string: true,
// 						from_string: true
// 					},
// 					{
// 						language: 'typescript',
// 						tags: ['easyapi'],
// 						type: 'number',
// 						to_string: 'value.toString()',
// 						from_string: 'parseInt(value)'
// 					},
// 					{
// 						language: 'graphql',
// 						tags: ['easyapi'],
// 						type: 'Int'
// 					}
// 				]
// 			}) as EasyApiImplementationType
// 	),
// 	...['f32', 'f64'].map(
// 		i =>
// 			({
// 				kind: 'implementation',
// 				name: i,
// 				namespace: 'std',
// 				description: `${i.replace(/[f]/, '')}-bit floating point number`,
// 				implementations: [
// 					{
// 						language: 'rust',
// 						tags: ['easyapi'],
// 						type: i,
// 						to_string: true,
// 						from_string: true
// 					},
// 					{
// 						language: 'typescript',
// 						tags: ['easyapi'],
// 						type: 'number',
// 						to_string: 'value.toString()',
// 						from_string: 'parseFloat(value)'
// 					},
// 					{
// 						language: 'graphql',
// 						tags: ['easyapi'],
// 						type: 'Float'
// 					}
// 				]
// 			}) as EasyApiImplementationType
// 	),
// 	{
// 		kind: 'implementation',
// 		name: 'option',
// 		namespace: 'std',
// 		parameters: ['T'],
// 		description: 'Nullable value',
// 		implementations: [
// 			{
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'Option<T>'
// 			},
// 			{
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'T | null'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: 'T | null' // the code generator will invert this to T!
// 			}
// 		]
// 	},
// 	{
// 		kind: 'implementation',
// 		name: 'vector',
// 		namespace: 'std',
// 		parameters: ['T'],
// 		description: 'Array of values',
// 		implementations: [
// 			{
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'Vec<T>'
// 			},
// 			{
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'Array<T>'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: '[T]'
// 			}
// 		]
// 	},
// 	{
// 		kind: 'implementation',
// 		name: 'hashmap',
// 		namespace: 'std',
// 		parameters: ['K', 'V'],
// 		description: 'Map of keys to values',
// 		implementations: [
// 			{
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'HashMap<K, V>'
// 			},
// 			{
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'Record<K, V>'
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: 'HashMap<K,V>' // hashmaps are not natively supported by graphql, so the generator has to worry about transformation
// 			}
// 		]
// 	},
// 	{
// 		kind: 'implementation',
// 		name: 'hashset',
// 		namespace: 'std',
// 		parameters: ['T'],
// 		description: 'Set of unique values',
// 		implementations: [
// 			{
// 				language: 'rust',
// 				tags: ['easyapi'],
// 				type: 'HashSet<T>'
// 			},
// 			{
// 				language: 'typescript',
// 				tags: ['easyapi'],
// 				type: 'Array<T>' // TODO implement converstion from / to Set<T>
// 			},
// 			{
// 				language: 'graphql',
// 				tags: ['easyapi'],
// 				type: '[T]'
// 			}
// 		]
// 	}
// ];
