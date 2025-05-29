mod messages;
mod db;
mod models;


fn main() {
    let topic = "topic:tech";
    let content_id = "hash_of_news_item_xyz"; // Could be hash(link+timestamp)

    // 1. Register this node as available to vote in this topic
    register_myself_to_topic(topic);

    // 2. User publishes a new content that needs voting
    let publisher_peer_id = get_own_peer_id();
    let vote_request_id = generate_vote_request_id(content_id);

    // 3. Discover potential voters from DHT or local registry
    let candidate_peers = get_peers_for_topic(topic); // from DHT

    // 4. Load local reputation DB and filter the best
    let filtered_voters = filter_voters_by_reputation(candidate_peers, min_reputation = 80.0);

    // 5. Select N nodes to form the voting group (e.g., 5-7)
    let voting_group = select_voting_group(filtered_voters, group_size = 5);

    // 6. Choose a leader (deterministic, e.g. highest rep or first in sorted list)
    let leader_peer_id = choose_leader_from_group(&voting_group);

    // 7. Create the vote request to send to the group
    let vote_request = VoteRequest {
        id: vote_request_id,
        content_id: content_id.to_string(),
        topic: topic.to_string(),
        publisher: publisher_peer_id.clone(),
        voters: voting_group.clone(),
        leader: leader_peer_id.clone(),
        ttl_secs: 30,
        signature: sign_with_private_key(...),
    };

    // 8. Send the vote request to the leader via libp2p::request_response
    send_vote_request_to_leader(&leader_peer_id, &vote_request);

    // 9. Locally mark this voting round as pending in Sled/SQLite
    mark_vote_as_pending_in_local_db(&vote_request);

    // 10. Listen for vote result from Gossipsub or DHT
    listen_for_vote_result(vote_request_id, |result| {
        validate_and_store_vote_result(result);
        update_local_reputation_based_on_result(result);
    });
}

fn register_myself_to_topic(topic: &str) {

}