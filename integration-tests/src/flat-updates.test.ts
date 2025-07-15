import { expect, test, suite } from 'vitest'
import { execute, getIntuition, pinJson, SystemAtom, wait } from './setup/utils.js'
import { graphql } from './graphql/gql.js'

suite('create person triple', () => {
  test('should create the person triple with all dependencies', async () => {

    const maya = await getIntuition(100)

    const mayaPerson = await maya.getOrCreateAtom(await pinJson({
      '@context': 'https://schema.org',
      '@type': 'Person',
      name: 'Maya',
      description: 'Product manager passionate about building great products',
      email: 'maya@nova-biotech.example',
      url: 'https://nova-biotech.example',
    }))
    await wait(mayaPerson.hash)

    const nameAtom = await maya.getOrCreateAtom('name')
    const newNameAtom = await maya.getOrCreateAtom('Maya Smiths')

    const triple = await maya.getCreateOrDepositOnTriple(
      mayaPerson.vaultId,
      nameAtom.vaultId,
      newNameAtom.vaultId,
    )
    await wait(triple.hash)

    expect(triple.vaultId).toBeDefined()
  })
})
