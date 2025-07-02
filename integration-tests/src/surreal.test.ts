import { expect, test, suite } from 'vitest'
import * as Automerge from "@automerge/automerge";
import { execute, getIntuition, pinJson, SystemAtom, wait } from './setup/utils.js'
import { graphql } from './graphql/gql.js'
import { PositionsSurrealDocument } from './graphql/graphql.js'
import { formatDuration, formatRelative, formatDistanceToNow } from 'date-fns';
import { RecordId, Surreal } from 'surrealdb';


const positionsCountQuery = graphql(`
query PositionsCountSurreal($where: positions_bool_exp) {
  positions_aggregate(where: $where) {
    aggregate {
      count
    }
  }
}
`);

const positionsQuery = graphql(`
query PositionsSurreal($limit: Int, $offset: Int, $where: positions_bool_exp) {
  positions(
    limit: $limit
    offset: $offset
    where: $where ) {
    shares
    account_id
    term {
      id
      atom {
        label
        type
        data
      }
      triple {
        subject {
          term_id
          label
          type
          data
        }
        predicate {
          term_id
          label
          type
          data
        }
        object {
          term_id
          label
          type
          data
        }
      }
    }
  }
}
`);

export interface PositionsResult {
  positions: {
    shares: number;
    account_id: string;
    term: {
      id: string;
      atom?: {
        label: string;
        type: string;
        data: string;
      };
      triple?: {
        subject: {
          term_id: string;
          label: string;
          type: string;
          data: string;
        };
        predicate: {
          term_id: string;
          label: string;
          type: string;
          data: string;
        };
        object: {
          term_id: string;
          label: string;
          type: string;
          data: string;
        };
      };
    };
  }[];
}




export const syncPositions = async (callback?: (positions: any) => Promise<void>, address?: string) => {
  // loop through all pages
  let offset = 0;
  let hasMore = true;
  const startTime = Date.now();

  const { positions_aggregate } = await execute(positionsCountQuery, { where: address ? { account_id: { _eq: address } } : undefined });
  const totalCount = positions_aggregate.aggregate.count;
  console.log('totalCount', totalCount);
  let processedCount = 0;
  let loopStartTime = Date.now();

  while (hasMore) {
    console.time('getPositions');
    const { positions } = await execute(positionsQuery, { where: address ? { account_id: { _eq: address } } : undefined, limit: 100, offset });
    console.timeEnd('getPositions');
    hasMore = processedCount < totalCount;
    offset += 100;

    const loopDuration = Date.now() - loopStartTime;
    const avgLoopTime = (Date.now() - startTime) / (processedCount / 100 || 1);
    const remainingLoops = Math.ceil((totalCount - processedCount) / 100);
    const estimatedEndTime = new Date(Date.now() + (avgLoopTime * remainingLoops));
    console.log('progress', processedCount, '/', totalCount, '(', Math.round((processedCount / totalCount) * 100), '%)');
    console.log('loop duration:', formatDuration({ seconds: Math.max(1, Math.floor(loopDuration / 1000)) }));
    if (isNaN(estimatedEndTime.getTime())) {
      console.log('estimated completion: calculating...');
    } else {
      console.log('estimated completion in', formatDistanceToNow(estimatedEndTime));
    }

    console.time('savePositions');
    await callback?.(positions);
    processedCount += positions.length;
    console.timeEnd('savePositions');

    loopStartTime = Date.now();
  }
  return true

};


test('should sync all positions', async () => {
  // const admin = await getIntuition(1)
  const db = new Surreal({});
  await db.connect('http://localhost:8069/rpc', {
    auth: {
      username: 'root',
      password: 'root',
    },
    database: 'intuition',
    namespace: 'intuition',
  });

  const sync = async (positions: PositionsResult['positions']) => {
    for (const position of positions) {
      if (position.term.atom) {
        const atom = await db.upsert(new RecordId('atom', position.term.id), {
          term_id: position.term.id,
          label: position.term.atom.label,
          atom_type: position.term.atom.type,
          data: position.term.atom.data,
        });
        // console.log(atom);
      }
      if (position.term.triple) {
        // upsert subject, predicate, object
        const subject = await db.upsert(new RecordId('atom', position.term.triple.subject.term_id), {
          term_id: position.term.triple.subject.term_id,
          label: position.term.triple.subject.label,
          atom_type: position.term.triple.subject.type,
          data: position.term.triple.subject.data,
        });
        const predicate = await db.upsert(new RecordId('atom', position.term.triple.predicate.term_id), {
          term_id: position.term.triple.predicate.term_id,
          label: position.term.triple.predicate.label,
          atom_type: position.term.triple.predicate.type,
          data: position.term.triple.predicate.data,
        });
        const object = await db.upsert(new RecordId('atom', position.term.triple.object.term_id), {
          term_id: position.term.triple.object.term_id,
          label: position.term.triple.object.label,
          atom_type: position.term.triple.object.type,
          data: position.term.triple.object.data,
        });

        // const triple = await db.upsert(new RecordId('triple', position.term.id), {
        //   in: new RecordId('atom', position.term.triple.subject.term_id),
        //   out: new RecordId('atom', position.term.triple.object.term_id),
        //   predicate_id: new RecordId('atom', position.term.triple.predicate.term_id),
        // });

        // Check if the relation already exists
        const relation: any = await db.query(`SELECT * FROM triple WHERE id = triple:\`${position.term.id}\``);
        if (relation[0].length > 0) {
          // update the relation
          // await db.query(`UPDATE triple:\`${position.term.id}\` SET holders += '${position.account_id}'`);
        } else {
          // create the relation
          await db.query(`RELATE atom:\`${position.term.triple.subject.term_id}\` -> triple:\`${position.term.id}\` -> atom:\`${position.term.triple.object.term_id}\`
          CONTENT {
            predicate: atom:\`${position.term.triple.predicate.term_id}\`
          }`);
        }


      }
    }
  }



  const result = await syncPositions(sync)
  expect(result).toEqual(true)

})
