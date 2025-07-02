import { expect, test, suite } from 'vitest'
import * as Automerge from "@automerge/automerge";
import { execute, getIntuition, pinJson, SystemAtom, wait } from './setup/utils.js'
import { graphql } from './graphql/gql.js'

suite('create person triple', () => {
  test('should create the person triple with all dependencies', async () => {
    const admin = await getIntuition(1)

    const change = await admin.getOrCreateAtom('automerge.change')
    await wait(change.hash)


    const maya = await getIntuition(11)
    const initialDocument = {
      '@context': 'https://schema.org',
      '@type': 'Person',
      name: 'Maya',
      description: 'Product manager passionate about building great products',
      email: 'maya@nova-biotech.example',
      url: 'https://nova-biotech.example',
    }

    const mayaPerson = await maya.getOrCreateAtom(await pinJson(initialDocument))
    await wait(mayaPerson.hash)


    // 1. Create an Automerge document from the initial state.
    // We use <any> because the schema is not known in advance.
    const doc1 = Automerge.from<any>(initialDocument);

    // 2. A user makes changes in a change block.
    const doc2 = Automerge.change(doc1, "", (doc) => {
      // We can now modify the document as if it were a regular JavaScript object.
      doc.name = "Maya Smith";
      doc.email = "maya.smith@nova-biotech.example";
      delete doc.url;
    });

    // 3. Get only the new binary changes that represent the modifications.
    // This is the "patch" you would send over the network.
    const [changeData] = Automerge.getChanges(doc1, doc2);
    console.log(Buffer.from(changeData).toString('hex'))

    const changeAtom = await maya.getOrCreateAtom(Buffer.from(changeData).toString())

    const triple = await maya.getCreateOrDepositOnTriple(
      mayaPerson.vaultId,
      change.vaultId,
      changeAtom.vaultId,
    )
    await wait(triple.hash)

    expect(change).toBeDefined()
    expect(changeAtom).toBeDefined()
    expect(mayaPerson).toBeDefined()
    expect(triple.vaultId).toBeDefined()
  })
})
