import { client } from './generated';

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
            kind: { type: 'dog', breed: 'Labrador' },
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

    console.log('streaming cdc events while mutating pets');
    const controller = new AbortController();
    const streamResult = await c.pets.cdc_events({}, {
        authorization: 'password'
    }, { signal: controller.signal });

    if (streamResult.is_err()) {
        console.log('stream error:', streamResult.unwrap_err().toString());
        return;
    }

    const received: string[] = [];
    const streamDone = (async () => {
        try {
            for await (const pet of streamResult.unwrap_ok()) {
                console.log('received event:', pet.name, JSON.stringify(pet.kind));
                received.push(pet.name);
            }
        } catch (e: any) {
            if (e.name !== 'AbortError') throw e;
        }
    })();

    await new Promise((r) => setTimeout(r, 100));

    console.log('creating Whiskers');
    (await c.pets.create({
        name: 'Whiskers',
        kind: { type: 'cat', lives: 9 },
        behaviors: [],
    }, { authorization: 'password' })).unwrap_ok();

    console.log('creating Tweety');
    (await c.pets.create({
        name: 'Tweety',
        kind: { type: 'bird' },
        behaviors: [],
    }, { authorization: 'password' })).unwrap_ok();

    console.log('updating Bobby');
    (await c.pets.update({
        name: 'Bobby',
        age: 2,
    }, { authorization: 'password' })).unwrap_ok();

    console.log('removing Whiskers');
    (await c.pets.remove({
        name: 'Whiskers',
    }, { authorization: 'password' })).unwrap_ok();

    await new Promise((r) => setTimeout(r, 500));
    controller.abort();
    await streamDone;

    const expected = ['Whiskers', 'Tweety', 'Bobby', 'Whiskers'];
    const ok = JSON.stringify(received) === JSON.stringify(expected);
    console.log(ok ? 'stream test passed' : `stream test FAILED: expected ${JSON.stringify(expected)}, got ${JSON.stringify(received)}`);

    console.log('removing remaining pets');
    (await c.pets.remove({ name: 'Bobby' }, { authorization: 'password' })).unwrap_ok();
    (await c.pets.remove({ name: 'Tweety' }, { authorization: 'password' })).unwrap_ok();
}

main()
    .then(() => console.log('done'))
    .catch((err) => console.error(err));
