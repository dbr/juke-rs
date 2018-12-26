class PlaybackStatus extends React.Component {
    state = {info: undefined}
    componentDidMount() {
        const sock = new WebSocket('ws://localhost:8081/ws', 'juke');
        sock.addEventListener('open', function (event) {
            sock.send('status');
        });
        sock.addEventListener('message', function (event) {
            let d = event.data;
            this.update(JSON.parse(d));
        }.bind(this));

        // Trigger routine update
        this.timer = setInterval(function(){sock.send('status')}.bind(this), 1000);
    }
    componentWillUnmount() {
        clearInterval(this.timer);
    }
    update(data) {
        this.setState({"info": data});
    }
    render() {
        if (this.state.info === undefined) {
            return <div>[loading...!]</div>;
        }
        return (
            <span>
                <p>
                    Playing: <b>{this.state.info.Status.song.title}</b> by <b>{this.state.info.Status.song.artist}</b>
                    <i> ({this.state.info.Status.state})</i>
                </p>
            </span>
        )
    }
}

ReactDOM.render(
    <div>
        <PlaybackStatus />
    </div>,
    document.getElementById("app")
);