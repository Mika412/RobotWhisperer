import { WSFoxgloveProvider } from '../lib/ros/wsFoxgloveProvider'
import { topics, services, actions, connection, latest } from './store'


export const provider = new WSFoxgloveProvider()
provider.setEvents({
    onStatus: s => connection.set({ ...s, source: 'ws' }),
    onTopics: t => topics.set(t),
    onServices: s => services.set(s),
    onActions: a => actions.set(a),
    onMessage: m => latest.update(L => (L[m.resourceName] = m, L))
})