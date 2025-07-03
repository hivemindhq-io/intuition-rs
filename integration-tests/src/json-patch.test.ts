import { expect, test, suite } from 'vitest'
import * as Automerge from "@automerge/automerge";
import { execute, getIntuition, pinJson, SystemAtom, wait } from './setup/utils.js'
import { graphql } from './graphql/gql.js'

suite('create person triple', () => {
  test('should create the person triple with all dependencies', async () => {
    const admin = await getIntuition(1)

    const patch_predicate = await admin.getOrCreateAtom('json_patch')
    await wait(patch_predicate.hash)


    const maya = await getIntuition(11)
    const initialJson = {
      '@context': 'https://schema.org',
      '@type': 'Person',
      name: 'Maya',
      description: 'Product manager passionate about building great products',
      email: 'maya@nova-biotech.example',
      url: 'https://nova-biotech.example',
    }

    const mayaPerson = await maya.getOrCreateAtom(await pinJson(initialJson))
    await wait(mayaPerson.hash)

    const patchJson =
      [
        { "op": "replace", "path": "/name", "value": "Maya Smith" },
        { "op": "add", "path": "/image", "value": "https://example.com/avatar.png" },
        { "op": "remove", "path": "/url" }
      ]

    const patchAtom = await maya.getOrCreateAtom(await pinJson(patchJson))

    const triple = await maya.getCreateOrDepositOnTriple(
      mayaPerson.vaultId,
      patch_predicate.vaultId,
      patchAtom.vaultId,
    )
    await wait(triple.hash)

    expect(patch_predicate).toBeDefined()
    expect(patchAtom).toBeDefined()
    expect(mayaPerson).toBeDefined()
    expect(triple.vaultId).toBeDefined()
  })
})
