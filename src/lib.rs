mod bindings;
mod state;
mod filesystem;
mod builder;
mod messaging;
mod error;

#[cfg(test)]
mod tests;

use bindings::exports::ntwk::theater::actor::Guest;
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClient;
use messaging::MessageHandler;

/// Main component implementation
struct Component;

impl Guest for Component {
    /// Initialize the build actor
    fn init(init_data: Option<bindings::exports::ntwk::theater::actor::Json>, params: (String,)) 
        -> Result<(Option<bindings::exports::ntwk::theater::actor::Json>,), String> {
        MessageHandler::init(init_data, params)
    }
}

impl MessageServerClient for Component {
    /// Handle send messages
    fn handle_send(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>, 
        params: (bindings::exports::ntwk::theater::message_server_client::Json,)
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String> {
        MessageHandler::handle_send(state, params)
    }

    /// Handle request messages
    fn handle_request(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (bindings::exports::ntwk::theater::message_server_client::Json,),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>, (bindings::exports::ntwk::theater::message_server_client::Json,)), String> {
        MessageHandler::handle_request(state, params)
    }

    fn handle_channel_open(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (bindings::exports::ntwk::theater::message_server_client::Json,),
    ) -> Result<
        (
            Option<bindings::exports::ntwk::theater::message_server_client::Json>,
            (bindings::exports::ntwk::theater::message_server_client::ChannelAccept,),
        ),
        String,
    > {
        MessageHandler::handle_channel_open(state, params)
    }

    fn handle_channel_close(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (String,),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        MessageHandler::handle_channel_close(state, params)
    }

    fn handle_channel_message(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (
            String,
            bindings::exports::ntwk::theater::message_server_client::Json,
        ),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        MessageHandler::handle_channel_message(state, params)
    }
}

// Export the component
bindings::export!(Component with_types_in bindings);
