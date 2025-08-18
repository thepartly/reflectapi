import { client } from './generated';

// workaround for node versions older than 18
import fetch from "node-fetch";
(globalThis.fetch as any) = fetch;

async function main() {
    const c = client('http://localhost:3000');

    console.log('checking health');
    (
        await c.health.check({}, {})
    ).unwrap_ok();

    console.log('creating pet');
    (
        await c.pets.create({
            name: 'Bobby',
            kind: { type: 'dog', breed: 'Golden Retriever' },
            age: 1,
            behaviors: [],
        }, {
            authorization: 'password'
        })
    ).unwrap_ok_or_else((e) => {
        const received_err = e.unwrap(); // throws if non application error
        switch (received_err) {
            case 'Conflict': {
                console.log('pet already exists');
                return {};
            }
            case 'NotAuthorized': {
                console.log('unauthorized');
                return {};
            }
            default: {
                console.log(received_err.InvalidIdentity.message);
                return {};
            }
        }
    });

    console.log('listing pets');
    const pets = (
        await c.pets.list({}, {
            authorization: 'password'
        })
    ).unwrap_ok();

    console.log(pets.items[0].name);

    console.log('removing pet');
    (
        await c.pets.remove({
            name: 'Bobby',
        }, {
            authorization: 'password'
        })
    ).unwrap_ok();
}

main()
    .then(() => console.log('done'))
    .catch((err) => console.error(err));
