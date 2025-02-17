pub fn silence_mollusk_prog_logs() {
    solana_logger::setup_with_default(
        "solana_rbpf::vm=warn,\
         solana_runtime::message_processor=warn,\
         solana_runtime::system_instruction_processor=warn",
    );
}
