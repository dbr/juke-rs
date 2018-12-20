class PlaybackStatus extends React.Component {
    state = {info: undefined}
    componentDidMount() {
        this.timer = setInterval(this.update.bind(this), 500);
    }
    componentWillUnmount() {
        clearInterval(this.timer);
    }
    update() {
        self = this;

        var u = new URL("http://localhost:8081/api/status");
        fetch(u).then(function (resp) {
            if (!resp.ok) {
                throw new Error('HTTP error, status = ' + response.status);
            }
            return resp.json()
        }).then(function(j) {
            self.setState({"info": j});
        })
    }
    render() {
        if(this.state.info === undefined){
            return <div>[unknown status!]</div>;
        }
        console.log("State", this.state);
        return (
            <span>
            <p>
                Playing: <b>{this.state.info.Status.song_title}</b> by <b>?</b>
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